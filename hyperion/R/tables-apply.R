# ==============================================================================
# Apply spec to parameter data
# ==============================================================================

#' Compute variability display string
#'
#' Creates formatted variability strings like "(CV = 35.8%)" from cv/corr/sd values.
#' Respects drop_columns - if cv/corr/sd are dropped, they won't appear.
#'
#' @param fixed Logical indicating if parameter is fixed
#' @param cv CV values
#' @param corr Correlation values
#' @param sd SD values
#' @param n_sigfig Number of significant figures
#' @param drop_columns Columns to exclude from variability display
#' @return Character vector of formatted variability strings
#' @noRd
compute_variability <- function(
  fixed,
  cv,
  corr,
  sd,
  n_sigfig,
  drop_columns
) {
  use_cv <- !"cv" %in% drop_columns
  use_corr <- !"corr" %in% drop_columns
  use_sd <- !"sd" %in% drop_columns

  dplyr::case_when(
    fixed ~ "(Fixed)",
    use_corr & !is.na(corr) ~
      sprintf("(Corr = %s)", format_hyperion_sigfig_string(corr, n_sigfig)),
    use_cv & !is.na(cv) & cv != 0 ~
      sprintf("(CV = %s%%)", format_hyperion_sigfig_string(cv, n_sigfig)),
    use_sd & !is.na(sd) ~
      sprintf("(SD = %s)", format_hyperion_sigfig_string(sd, n_sigfig)),
    TRUE ~ NA_character_
  )
}

#' Apply table specification to parameter data
#'
#' Enriches parameter data with transforms, CIs, sections, and display names.
#'
#' @param params Data frame from get_parameters()
#' @param spec A TableSpec object
#' @param info ModelComments object from get_model_parameter_info(), or NULL.
#'   If NULL, features that require ModelComments (transforms, units,
#'   descriptions, custom name sources) will not be available and warnings
#'   will be issued if requested.
#' @importFrom rlang .data
#'
#' @return Enriched data frame ready for table building
#' @export
apply_table_spec <- function(params, spec, info = NULL) {
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    stop("Package 'dplyr' is required for apply_table_spec()")
  }
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }
  if (!is.null(info) && !S7::S7_inherits(info, ModelComments)) {
    stop("info must be a ModelComments object or NULL")
  }

  dt_kinds <- build_display_transforms(spec)
  col_values <- unlist(spec@display_transforms)

  # Build dt_* column expressions
  dt_exprs <- lapply(names(dt_kinds), function(group) {
    kinds <- dt_kinds[[group]]
    rlang::expr(dplyr::if_else(
      .data$kind %in% !!kinds,
      .data$transforms,
      "identity"
    ))
  }) |>
    stats::setNames(paste0("dt_", names(dt_kinds)))

  # Helper to get the right dt column for a given output column
  dt_for <- function(col) {
    if (col %in% col_values) paste0("dt_", col) else "dt_all"
  }

  # Handle transforms and unit based on whether info is provided
  if (!is.null(info)) {
    transforms_vec <- get_parameter_transform(info, params$name, params$kind)
    unit_vec <- get_parameter_unit(info, params$name, params$kind)
  } else {
    transforms_vec <- rep("identity", nrow(params))
    unit_vec <- rep(NA_character_, nrow(params))
  }

  df <- params |>
    dplyr::mutate(
      transforms = transforms_vec,
      unit = unit_vec,
      !!!dt_exprs,
      cv = compute_cv(.data$estimate, .data$kind, .data[[dt_for("cv")]]),
      rse = compute_rse(
        .data$estimate,
        .data$stderr,
        .data$kind,
        .data[[dt_for("rse")]]
      ),
      ci_low = compute_ci(
        .data$estimate,
        .data$stderr,
        spec@ci_level,
        .data[[dt_for("ci")]]
      )$lower,
      ci_high = compute_ci(
        .data$estimate,
        .data$stderr,
        spec@ci_level,
        .data[[dt_for("ci")]]
      )$upper,
      estimate = transform_value(.data$estimate, .data[[dt_for("estimate")]]),
      symbol = param_symbol_md(
        .data$kind,
        .data$random_effect,
        .data[[dt_for("symbol")]]
      ),
      variability = compute_variability(
        .data$fixed,
        .data$cv,
        .data$corr,
        .data$sd,
        spec@n_sigfig,
        spec@drop_columns
      )
    )

  # Add description column FIRST (before name transformation)
  # This ensures we match on original/untransformed names
  want_description <- "description" %in%
    c(spec@columns, spec@add_columns %||% character(0)) &&
    !"description" %in% spec@drop_columns
  if (want_description) {
    if (is.null(info)) {
      warning(
        "description requires a ModelComments object. ",
        "Descriptions will not be available.",
        call. = FALSE
      )
      df$description <- NA_character_
    } else {
      df <- enrich_description(df, info)
    }
  }

  # Add nonmem_name and user_name columns for filtering/sectioning
  if (!is.null(info)) {
    # get_parameter_names returns df with rownames = nonmem_name, columns = name, display
    labels <- get_parameter_names(info)

    # Match params to ModelComments by the current name (could be nonmem or user name)
    match_idx <- match(df$name, rownames(labels)) # Try nonmem_name first
    if (all(is.na(match_idx))) {
      match_idx <- match(df$name, labels$name) # Try user_name
    }

    df$nonmem_name <- rownames(labels)[match_idx]
    df$user_name <- labels$name[match_idx]

    # Fallback to current name if no match
    df$nonmem_name <- ifelse(is.na(df$nonmem_name), df$name, df$nonmem_name)
    df$user_name <- ifelse(is.na(df$user_name), df$name, df$user_name)
  } else {
    # No ModelComments - use current name for both
    df$nonmem_name <- df$name
    df$user_name <- df$name
  }

  # Apply name replacement based on spec@name_source
  if (!is.null(info)) {
    df <- apply_name_source(df, info, spec@name_source)
  } else if (spec@name_source != "nonmem_name") {
    warning(
      "name_source '",
      spec@name_source,
      "' requires a ModelComments object. ",
      "Using NONMEM names instead.",
      call. = FALSE
    )
  }

  # Apply section rules AFTER name transformation (consistent with row_filter)
  df <- df |>
    dplyr::mutate(
      section = build_section(dplyr::pick(dplyr::everything()), spec)
    )

  # Apply row filter AFTER name transformation so users can filter on display names
  if (length(spec@row_filter) > 0) {
    for (f in spec@row_filter) {
      df <- df |>
        dplyr::filter(!!f)
    }
  }

  attr(df, "table_spec") <- spec
  df
}

# ==============================================================================
# TableSpec helper functions
# ==============================================================================

#' Build display transform mapping from spec
#' @noRd
build_display_transforms <- function(spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  dt <- spec@display_transforms
  groups <- unique(unlist(dt))

  dt_kinds <- lapply(groups, function(group) {
    kinds <- names(dt)[vapply(
      dt,
      function(x) {
        !is.null(x) && ("all" %in% x || group %in% x)
      },
      logical(1)
    )]
    toupper(kinds)
  }) |>
    stats::setNames(groups)

  # Always provide dt_all as a fallback transform mapping for every kind
  if (!"all" %in% names(dt_kinds)) {
    dt_kinds[["all"]] <- toupper(names(dt))
  }

  dt_kinds
}

#' Build section assignments using case_when
#' @noRd
build_section <- function(data, spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  rules <- spec@sections
  if (length(rules) == 0) {
    return(rep(NA_character_, nrow(data)))
  }

  # Convert quosures to case_when format
  # Each quosure wraps a formula like: kind == "THETA" ~ "Structural model parameters"
  args <- lapply(rules, function(q) {
    rlang::eval_tidy(q, data = data)
  })

  dplyr::case_when(!!!args)
}

#' Get section order from spec
#' @noRd
get_section_order <- function(spec) {
  if (!S7::S7_inherits(spec, TableSpec)) {
    stop("spec must be a TableSpec object")
  }

  vapply(
    spec@sections,
    function(rule) {
      rlang::f_rhs(rlang::eval_tidy(rule))
    },
    character(1)
  )
}

#' @noRd
comment_keys_for <- function(nonmem, comment, include_associated_theta = TRUE) {
  keys <- c(nonmem)

  if (!is.null(comment@name)) {
    keys <- c(keys, comment@name)

    if (
      include_associated_theta &&
        S7::S7_inherits(comment, OmegaComment) &&
        !is.null(comment@associated_theta)
    ) {
      theta_str <- paste(comment@associated_theta, collapse = "-")
      keys <- c(keys, paste0(comment@name, " (", theta_str, ")"))
    }
  }

  if (!is.null(comment@display)) {
    keys <- c(keys, comment@display)
  }

  keys
}

#' @noRd
build_name_lookup <- function(info, name_source) {
  labels <- get_parameter_names(info)

  build_lookup_rows <- function(comments, kind_label) {
    lapply(names(comments), function(nonmem) {
      cmt <- comments[[nonmem]]

      if (!nonmem %in% rownames(labels)) {
        target <- nonmem
      } else if (name_source == "nonmem_name") {
        target <- nonmem
      } else if (
        name_source == "display" && !is.na(labels[nonmem, "display"])
      ) {
        target <- labels[nonmem, "display"]
      } else if (!is.na(labels[nonmem, "name"])) {
        target <- labels[nonmem, "name"]
      } else {
        target <- nonmem
      }

      # For omega, build composite display name if associated_theta not in target
      if (
        S7::S7_inherits(cmt, OmegaComment) &&
          !is.null(cmt@associated_theta)
      ) {
        if (name_source == "nonmem_name") {
          theta_labels <- vapply(
            cmt@associated_theta,
            function(theta_name) {
              theta_row <- which(labels$name == theta_name)
              if (length(theta_row) > 0) {
                rownames(labels)[theta_row[1]]
              } else {
                theta_name
              }
            },
            character(1)
          )
          theta_already_present <- vapply(
            theta_labels,
            function(theta_label) {
              grepl(theta_label, target, fixed = TRUE)
            },
            logical(1)
          )
        } else {
          # Check if theta info is already present (by name or display)
          theta_already_present <- vapply(
            cmt@associated_theta,
            function(theta_name) {
              # Check if theta name itself is in target
              if (grepl(theta_name, target, fixed = TRUE)) return(TRUE)
              # Find theta's display name (labels has rownames=nonmem_name, name column)
              theta_row <- which(labels$name == theta_name)
              if (length(theta_row) > 0) {
                theta_display <- labels$display[theta_row[1]]
                if (
                  !is.na(theta_display) &&
                    grepl(theta_display, target, fixed = TRUE)
                ) {
                  return(TRUE)
                }
              }
              FALSE
            },
            logical(1)
          )
          theta_labels <- cmt@associated_theta
        }

        # Only append missing thetas to avoid duplication
        missing_thetas <- theta_labels[!theta_already_present]
        if (length(missing_thetas) > 0) {
          theta_str <- paste(missing_thetas, collapse = "-")
          target <- paste0(target, "-", theta_str)
        }
      }

      keys <- comment_keys_for(nonmem, cmt, include_associated_theta = TRUE)

      data.frame(
        key = keys,
        display = target,
        kind = kind_label,
        stringsAsFactors = FALSE
      )
    }) |>
      dplyr::bind_rows()
  }

  dplyr::bind_rows(
    build_lookup_rows(info@theta, "THETA"),
    build_lookup_rows(info@omega, "OMEGA"),
    build_lookup_rows(info@sigma, "SIGMA")
  ) |>
    dplyr::distinct(.data$key, .data$kind, .keep_all = TRUE)
}

#' Apply name source replacement
#'
#' Replaces parameter names based on the name_source setting.
#'
#' @param df Data frame with name and kind columns
#' @param info ModelComments object
#' @param name_source "name", "display", or "nonmem_name"
#' @return Data frame with names replaced
#' @noRd
apply_name_source <- function(df, info, name_source) {
  lookup <- build_name_lookup(info, name_source)

  df |>
    dplyr::mutate(
      .match_idx = match(
        paste(.data$name, .data$kind),
        paste(lookup$key, lookup$kind)
      ),
      .display = lookup$display[.data$.match_idx],
      name = dplyr::coalesce(.data$.display, .data$name)
    ) |>
    dplyr::select(-".match_idx", -".display")
}

#' Enrich description column from ModelComments
#'
#' Adds a description column by matching parameter names to ModelComments.
#'
#' @param df Data frame with name and kind columns
#' @param info ModelComments object
#' @return Data frame with description column added
#' @noRd
enrich_description <- function(df, info) {
  build_desc_rows <- function(comments, kind_label) {
    lapply(names(comments), function(nonmem) {
      cmt <- comments[[nonmem]]
      desc <- cmt@description
      if (is.null(desc)) desc <- NA_character_

      keys <- comment_keys_for(nonmem, cmt, include_associated_theta = TRUE)

      data.frame(
        key = keys,
        description = desc,
        kind = kind_label,
        stringsAsFactors = FALSE
      )
    }) |>
      dplyr::bind_rows()
  }

  lookup <- dplyr::bind_rows(
    build_desc_rows(info@theta, "THETA"),
    build_desc_rows(info@omega, "OMEGA"),
    build_desc_rows(info@sigma, "SIGMA")
  ) |>
    dplyr::distinct(.data$key, .data$kind, .keep_all = TRUE)

  df |>
    dplyr::mutate(
      .match_idx = match(
        paste(.data$name, .data$kind),
        paste(lookup$key, lookup$kind)
      ),
      description = lookup$description[.data$.match_idx]
    ) |>
    dplyr::select(-".match_idx")
}
