# ==============================================================================
# Flextable Rendering for HyperionTable
# ==============================================================================

#' Render HyperionTable as flextable
#'
#' Converts a HyperionTable intermediate representation to a flextable object.
#'
#' @param table A HyperionTable object
#' @return A flextable object
#' @export
render_flextable <- function(table) {
  if (!S7::S7_inherits(table, HyperionTable)) {
    stop("table must be a HyperionTable object")
  }
  if (!requireNamespace("flextable", quietly = TRUE)) {
    stop("Package 'flextable' is required for render_flextable()")
  }

  payload <- prepare_table_render(table)
  data <- payload$data
  visible_cols <- payload$visible_cols

  # Handle row groups
  if (!is.null(table@groupname_col) && table@groupname_col %in% names(data)) {
    ft <- create_grouped_flextable(data, table)
  } else {
    ft <- flextable::flextable(data)
  }

  # Render symbol columns as equations
  ft <- apply_flextable_symbol_equations(ft, table, visible_cols)

  # Apply column labels
  ft <- apply_flextable_labels(ft, table, visible_cols)

  # Apply spanners
  ft <- apply_flextable_spanners(ft, table, visible_cols)

  # Apply bold styling
  ft <- apply_flextable_bold(ft, table)

  # Apply borders
  ft <- apply_flextable_borders(ft, table, visible_cols)

  # Add title
  ft <- apply_flextable_title(ft, table)

  # Add footnotes
  ft <- apply_flextable_footnotes(ft, table)

  # Auto-fit columns
  ft <- flextable::autofit(ft)

  ft
}

# ==============================================================================
# Flextable Creation Helpers
# ==============================================================================
#' Create grouped flextable
#'
#' @param data Data frame
#' @param table HyperionTable object
#' @return flextable object with row groups
#' @noRd
create_grouped_flextable <- function(data, table) {
  group_col <- table@groupname_col

  # Use flextable's as_grouped_data for row grouping
  grouped_data <- flextable::as_grouped_data(data, groups = group_col)

  # Identify group rows (rows where the group column has content)
  group_rows <- which(
    !is.na(grouped_data[[group_col]]) &
      grouped_data[[group_col]] != ""
  )

  # Create flextable
  ft <- flextable::flextable(grouped_data)

  # Get total number of columns
  n_cols <- ncol(grouped_data)

  # Merge cells in group rows to span full width and show group name
  for (row_idx in group_rows) {
    ft <- flextable::merge_h(ft, i = row_idx, part = "body")
  }

  # Bold the group rows
  if (length(group_rows) > 0) {
    ft <- flextable::bold(ft, i = group_rows, bold = TRUE)
  }

  # Hide the section column header
  ft <- flextable::void(ft, j = group_col, part = "header")
  ft <- flextable::width(ft, j = group_col, width = 0.01)

  ft
}

#' Apply symbol equation rendering to flextable
#'
#' Renders symbol columns as equations using as_equation() for proper
#' mathematical formatting with subscripts.
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @param visible_cols Character vector of visible column names
#' @return flextable with equations in symbol columns
#' @noRd
apply_flextable_symbol_equations <- function(ft, table, visible_cols) {
  # Find symbol columns in original data (symbol, symbol_1, symbol_2, etc.)
  symbol_cols <- grep("^symbol(_[0-9]+)?$", names(table@data), value = TRUE)
  visible_symbol_cols <- intersect(symbol_cols, visible_cols)

  if (length(visible_symbol_cols) == 0) {
    return(ft)
  }

  # Determine row indices (for grouped tables, data rows shift)
  if (
    !is.null(table@groupname_col) &&
      table@groupname_col %in% names(ft$body$dataset)
  ) {
    grouped_data <- ft$body$dataset
    group_col <- table@groupname_col
    data_rows <- which(
      is.na(grouped_data[[group_col]]) |
        grouped_data[[group_col]] == ""
    )
  } else {
    data_rows <- NULL
  }

  # Render each symbol column from original data
  for (col in visible_symbol_cols) {
    ft <- render_symbol_equations(ft, table@data[[col]], col, data_rows)
  }

  ft
}

#' Render symbol column as equations
#'
#' Uses as_equation() to render LaTeX symbols with proper subscripts.
#'
#' @param ft flextable object
#' @param original_symbol Character vector of original LaTeX symbol values
#' @param symbol_col Name of symbol column
#' @param row_indices Optional vector of row indices in the flextable corresponding
#'   to the original data rows (used for grouped tables where row indices shift)
#' @return flextable with equations in symbol column
#' @noRd
render_symbol_equations <- function(
  ft,
  original_symbol,
  symbol_col = "symbol",
  row_indices = NULL
) {
  if (is.null(original_symbol)) return(ft)
  if (!requireNamespace("equatags", quietly = TRUE)) return(ft)

  col_idx <- which(names(ft$body$dataset) == symbol_col)
  if (length(col_idx) == 0) return(ft)

  # Default: row indices match 1:n

  if (is.null(row_indices)) {
    row_indices <- seq_along(original_symbol)
  }

  for (i in seq_along(original_symbol)) {
    val <- original_symbol[i]
    if (is.na(val) || !grepl("\\$", val)) next

    # Extract LaTeX from $...$
    latex <- gsub("^\\$|\\$$", "", val)

    ft <- flextable::compose(
      ft,
      i = row_indices[i],
      j = col_idx,
      part = "body",
      value = flextable::as_paragraph(
        flextable::as_equation(latex, width = 0.5, height = 0.18)
      )
    )
  }

  ft
}

#' Apply column labels to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @param visible_cols Character vector of visible column names
#' @return flextable object with labels applied
#' @noRd
apply_flextable_labels <- function(ft, table, visible_cols) {
  if (length(table@col_labels) == 0) {
    return(ft)
  }

  # Filter to visible columns only
  labels <- table@col_labels[intersect(names(table@col_labels), visible_cols)]

  # Convert markdown/LaTeX-ish labels to plain text for flextable
  labels <- lapply(labels, function(x) {
    convert_md_label(as.character(x))
  })

  if (length(labels) > 0) {
    ft <- flextable::set_header_labels(ft, values = labels)
  }

  ft
}

#' Convert markdown label to plain text
#' @noRd
convert_md_label <- function(x) {
  # Convert LaTeX Delta to Unicode
  result <- gsub("\\$\\\\Delta\\$", "\u0394", x)
  # Remove remaining $ delimiters
  result <- gsub("\\$", "", result)
  result
}

#' Apply spanners to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @param visible_cols Character vector of visible column names
#' @return flextable object with spanners applied
#' @noRd
apply_flextable_spanners <- function(ft, table, visible_cols) {
  if (length(table@spanners) == 0) {
    return(ft)
  }

  # Build spanner row values and colwidths
  # First, get current header labels
  header_labels <- ft$header$dataset[1, ]

  # Map visible columns to their spanner (or empty string if no spanner)
  spanner_values <- rep("", length(visible_cols))
  names(spanner_values) <- visible_cols

  for (spanner in table@spanners) {
    cols <- intersect(spanner$columns, visible_cols)
    for (col in cols) {
      if (col %in% names(spanner_values)) {
        spanner_values[col] <- spanner$label
      }
    }
  }

  # Only add spanner row if there are actual spanners
  if (any(spanner_values != "")) {
    ft <- flextable::add_header_row(
      ft,
      values = as.character(spanner_values),
      colwidths = rep(1, length(spanner_values)),
      top = TRUE
    )

    # Merge adjacent cells with same spanner label
    ft <- flextable::merge_h(ft, part = "header")

    # Bold spanner row
    ft <- flextable::bold(ft, i = 1, part = "header")
  }

  ft
}

#' Apply bold styling to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @return flextable object with bold applied
#' @noRd
apply_flextable_bold <- function(ft, table) {
  if ("column_labels" %in% table@bold_locations) {
    # Get header row count
    n_header_rows <- nrow(ft$header$dataset)
    # Bold the last header row (column labels)
    ft <- flextable::bold(ft, i = n_header_rows, part = "header")
  }

  # Note: row_groups are handled separately in their creation

  ft
}

#' Apply title to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @return flextable object with title applied
#' @noRd
apply_flextable_title <- function(ft, table) {
  if (is.null(table@title) || nchar(table@title) == 0) {
    return(ft)
  }

  ft <- flextable::add_header_lines(ft, values = table@title, top = TRUE)
  ft <- flextable::bold(ft, i = 1, part = "header")

  ft
}

#' Apply borders to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @param visible_cols Character vector of visible column names
#' @return flextable object with borders applied
#' @noRd
apply_flextable_borders <- function(ft, table, visible_cols) {
  for (border in table@borders) {
    cols <- intersect(border$columns, visible_cols)
    if (length(cols) == 0) next

    # Map column names to indices
    col_indices <- which(visible_cols %in% cols)

    if ("right" %in% border$sides) {
      for (idx in col_indices) {
        ft <- flextable::vline(
          ft,
          j = idx,
          border = officer::fp_border(color = border$color),
          part = "all"
        )
      }
    }

    if ("left" %in% border$sides) {
      for (idx in col_indices) {
        ft <- flextable::vline(
          ft,
          j = idx - 1,
          border = officer::fp_border(color = border$color),
          part = "all"
        )
      }
    }

    if ("bottom" %in% border$sides) {
      ft <- flextable::hline(
        ft,
        j = col_indices,
        border = officer::fp_border(color = border$color),
        part = "body"
      )
    }
  }

  ft
}

#' Apply footnotes to flextable
#'
#' @param ft flextable object
#' @param table HyperionTable object
#' @return flextable object with footnotes applied
#' @noRd
apply_flextable_footnotes <- function(ft, table) {
  use_equations <- requireNamespace("equatags", quietly = TRUE)

  for (fn in table@footnotes) {
    content <- fn$content

    # Check if this footnote contains equations and we can render them
    has_equations <- fn$is_markdown && grepl("\\$", content)

    if (has_equations && use_equations) {
      # Build equation string with \text{} for non-math parts
      equation_str <- convert_to_single_equation(content)

      # Calculate width based on content length (rough heuristic)
      eq_width <- min(7, max(2, nchar(content) * 0.05))

      # add_footer_lines accepts as_paragraph directly as values
      ft <- flextable::add_footer_lines(
        ft,
        values = flextable::as_paragraph(
          flextable::as_equation(equation_str, width = eq_width, height = 0.2)
        )
      )
    } else if (fn$is_markdown) {
      # Fallback: convert to text with subscripts
      paragraph <- build_footnote_paragraph(content)
      ft <- flextable::add_footer_lines(ft, values = paragraph)
    } else {
      ft <- flextable::add_footer_lines(ft, values = content)
    }
  }

  ft
}

#' Convert footnote content to a single LaTeX equation string
#'
#' Wraps non-math text in \\text{} so the whole footnote can be rendered
#' as a single equation.
#'
#' @param content Character string with $...$ LaTeX equations
#' @return Single LaTeX equation string (without outer $ delimiters)
#' @noRd
convert_to_single_equation <- function(content) {
  equation_parts <- character(0)
  remaining <- content

  while (nzchar(remaining)) {
    # Find next $
    start <- regexpr("\\$", remaining)

    if (start == -1) {
      # No more equations, wrap remaining text in \text{}
      if (nzchar(remaining)) {
        text_escaped <- gsub("%", "\\\\%", remaining)
        equation_parts <- c(equation_parts, sprintf("\\text{%s}", text_escaped))
      }
      break
    }

    # Add text before the equation (wrapped in \text{})
    if (start > 1) {
      before <- substr(remaining, 1, start - 1)
      text_escaped <- gsub("%", "\\\\%", before)
      equation_parts <- c(equation_parts, sprintf("\\text{%s}", text_escaped))
    }

    # Find closing $
    after_start <- substr(remaining, start + 1, nchar(remaining))
    end <- regexpr("\\$", after_start)

    if (end == -1) {
      # No closing $, wrap rest as text
      text_escaped <- gsub(
        "%",
        "\\\\%",
        substr(remaining, start, nchar(remaining))
      )
      equation_parts <- c(equation_parts, sprintf("\\text{%s}", text_escaped))
      break
    }

    # Extract equation content (without $ delimiters) - keep as-is
    equation <- substr(after_start, 1, end - 1)
    equation_parts <- c(equation_parts, equation)

    # Move past this equation
    remaining <- substr(after_start, end + 1, nchar(after_start))
  }

  paste(equation_parts, collapse = "")
}

#' Build a formatted footnote paragraph with subscripts
#'
#' Parses LaTeX content and creates a flextable paragraph with proper
#' subscript formatting using as_sub().
#'
#' @param content Character string with LaTeX
#' @return A flextable as_paragraph object
#' @noRd
build_footnote_paragraph <- function(content) {
  # Convert LaTeX to text with subscripts using as_sub()
  result <- content

  # Handle specific complex formulas with nested braces FIRST
  # CV% formula: sqrt(exp(Estimate) - 1) × 100
  result <- gsub(
    "\\$\\\\sqrt\\{\\\\exp\\(\\\\mathrm\\{Estimate\\}\\) - 1\\} \\\\times 100\\$",
    "sqrt(exp(Estimate) - 1) \u00D7 100",
    result
  )
  # CV% formula variant with ^2: sqrt(exp(Estimate^2) - 1) × 100
  result <- gsub(
    "\\$\\\\sqrt\\{\\\\exp\\(\\\\mathrm\\{Estimate\\}\\^2\\) - 1\\} \\\\times 100\\$",
    "sqrt(exp(Estimate\u00B2) - 1) \u00D7 100",
    result
  )
  # % Change formula: (Estimate_model - Estimate_ref) / Estimate_ref · 100
  result <- gsub(
    "\\$\\\\frac\\{\\\\mathrm\\{Estimate\\}_\\{\\\\mathrm\\{model\\}\\} - \\\\mathrm\\{Estimate\\}_\\{\\\\mathrm\\{ref\\}\\}\\}\\{\\\\mathrm\\{Estimate\\}_\\{\\\\mathrm\\{ref\\}\\}\\} \\\\cdot 100\\$",
    "(Estimate_{model} - Estimate_{ref}) / Estimate_{ref} \u00B7 100",
    result
  )

  # Greek letters
  result <- gsub("\\\\theta", "\u03B8", result)
  result <- gsub("\\\\Theta", "\u0398", result)
  result <- gsub("\\\\Omega", "\u03A9", result)
  result <- gsub("\\\\omega", "\u03C9", result)
  result <- gsub("\\\\Sigma", "\u03A3", result)
  result <- gsub("\\\\sigma", "\u03C3", result)
  result <- gsub("\\\\Delta", "\u0394", result)
  result <- gsub("\\\\delta", "\u03B4", result)

  # Math operators
  result <- gsub("\\\\cdot", "\u00B7", result)
  result <- gsub("\\\\times", "\u00D7", result)
  result <- gsub("\\\\pm", "\u00B1", result)
  result <- gsub("\\\\sqrt\\{([^}]+)\\}", "sqrt(\\1)", result)
  result <- gsub("\\\\frac\\{([^}]+)\\}\\{([^}]+)\\}", "(\\1)/(\\2)", result)
  result <- gsub("\\\\exp\\(([^)]+)\\)", "exp(\\1)", result)
  result <- gsub("\\\\exp", "exp", result)
  result <- gsub("\\\\mathrm\\{([^}]+)\\}", "\\1", result)

  # Remove $ delimiters
  result <- gsub("\\$", "", result)

  # Now parse for subscripts and build paragraph chunks
  chunks <- parse_subscripts_to_chunks(result)

  # Build the paragraph from chunks
  do.call(flextable::as_paragraph, chunks)
}

#' Parse text with subscripts into flextable chunks
#'
#' @param text Text with _{...} subscript notation
#' @return List of flextable chunk objects
#' @noRd
parse_subscripts_to_chunks <- function(text) {
  chunks <- list()

  # Pattern to find subscripts: _{...}
  pattern <- "_\\{([^}]+)\\}"

  remaining <- text
  while (nzchar(remaining)) {
    match <- regexpr(pattern, remaining, perl = TRUE)

    if (match == -1) {
      # No more subscripts, add remaining text
      if (nzchar(remaining)) {
        chunks <- c(chunks, list(flextable::as_chunk(remaining)))
      }
      break
    }

    # Add text before the subscript
    if (match > 1) {
      before <- substr(remaining, 1, match - 1)
      chunks <- c(chunks, list(flextable::as_chunk(before)))
    }

    # Extract and add the subscript content
    match_length <- attr(match, "match.length")
    subscript_full <- substr(remaining, match, match + match_length - 1)
    subscript_content <- gsub(pattern, "\\1", subscript_full, perl = TRUE)
    chunks <- c(chunks, list(flextable::as_sub(subscript_content)))

    # Move past this match
    remaining <- substr(remaining, match + match_length, nchar(remaining))
  }

  if (length(chunks) == 0) {
    chunks <- list(flextable::as_chunk(text))
  }

  chunks
}

#' Convert markdown/LaTeX footnote to plain text
#'
#' @param content Character string with markdown/LaTeX
#' @return Plain text version
#' @noRd
convert_footnote_to_text <- function(content) {
  result <- content

  # Greek letters
  result <- gsub("\\\\theta", "\u03B8", result)
  result <- gsub("\\\\Theta", "\u0398", result)
  result <- gsub("\\\\Omega", "\u03A9", result)
  result <- gsub("\\\\omega", "\u03C9", result)
  result <- gsub("\\\\Sigma", "\u03A3", result)
  result <- gsub("\\\\sigma", "\u03C3", result)
  result <- gsub("\\\\Delta", "\u0394", result)
  result <- gsub("\\\\delta", "\u03B4", result)

  # Math operators
  result <- gsub("\\\\cdot", "\u00B7", result)
  result <- gsub("\\\\times", "\u00D7", result)
  result <- gsub("\\\\pm", "\u00B1", result)
  result <- gsub("\\\\sqrt\\{([^}]+)\\}", "sqrt(\\1)", result)
  result <- gsub("\\\\frac\\{([^}]+)\\}\\{([^}]+)\\}", "(\\1)/(\\2)", result)
  result <- gsub("\\\\exp\\(([^)]+)\\)", "exp(\\1)", result)
  result <- gsub("\\\\exp", "exp", result)
  result <- gsub("\\\\mathrm\\{([^}]+)\\}", "\\1", result)

  # Subscripts - convert to parenthetical notation
  result <- gsub("_\\{([^}]+)\\}", "(\\1)", result)

  # Remove $ delimiters
  result <- gsub("\\$", "", result)

  # Clean up extra spaces
  result <- gsub("\\s+", " ", result)
  result <- trimws(result)

  result
}

# ==============================================================================
# Summary Table Special Handling
# ==============================================================================

#' Render summary HyperionTable as flextable
#'
#' Specialized rendering for summary tables with additional formatting.
#'
#' @param table A HyperionTable object with table_type = "summary"
#' @return A flextable object
#' @noRd
render_flextable_summary <- function(table) {
  if (!S7::S7_inherits(table, HyperionTable)) {
    stop("table must be a HyperionTable object")
  }
  if (table@table_type != "summary") {
    stop("table@table_type must be 'summary'")
  }

  spec <- table@source_spec
  data <- table@data

  # Pre-merge pvalue with df before creating table
  if (all(c("pvalue", "df") %in% names(data))) {
    use_scientific <- spec@pvalue_scientific
    pval_threshold <- spec@pvalue_threshold
    n_sig <- spec@n_sigfig
    data$pvalue <- vapply(
      seq_len(nrow(data)),
      function(i) {
        pval <- data$pvalue[i]
        df_val <- data$df[i]
        if (is.na(pval) || is.na(df_val)) {
          return(NA_character_)
        }
        format_p <- format_pvalue_string(
          pval,
          n_sig,
          use_scientific,
          pval_threshold
        )
        sprintf("%s (df = %d)", format_p, df_val)
      },
      character(1)
    )
    data$df <- NULL
    table@data <- data
  }

  # Format OFV columns with specific decimals
  ofv_cols <- intersect(c("ofv", "dofv"), names(data))
  for (col in ofv_cols) {
    if (is.numeric(data[[col]])) {
      data[[col]] <- vapply(
        data[[col]],
        function(x) {
          if (is.na(x)) return("")
          formatC(x, digits = spec@n_decimals_ofv, format = "f")
        },
        character(1)
      )
    }
  }
  table@data <- data

  # Remove OFV cols from numeric_cols to avoid double formatting
  table@numeric_cols <- setdiff(table@numeric_cols, ofv_cols)

  # Use standard rendering
  render_flextable(table)
}
