#' @noRd
build_tree_display_parts <- function(x) {
  if (is.null(x$nodes) || length(x$nodes) == 0) {
    return(list(
      is_empty = TRUE,
      title = "Hyperion Model Tree"
    ))
  }

  tree_data <- build_cli_tree_data(x)
  total_models <- length(tree_data$parent)
  all_parents <- tree_data$parent
  all_children <- unlist(tree_data$children)
  root_nodes <- setdiff(all_parents, all_children)

  list(
    is_empty = FALSE,
    title = "Hyperion Model Tree",
    tree_data = tree_data,
    total_models = total_models,
    root_nodes = root_nodes,
    nodes = x$nodes
  )
}

#' Print Method for Hyperion Tree Objects
#'
#' Displays a hyperion_nonmem_tree in a readable tree format using cli::tree().
#' Shows the hierarchical relationships between models with Unicode tree characters.
#'
#' @param x A hyperion_nonmem_tree object
#' @param ... Additional arguments (currently unused)
#'
#' @return Invisibly returns the input object
#' @rawNamespace S3method(base::print, hyperion_nonmem_tree)
print.hyperion_nonmem_tree <- function(x, ...) {
  cli::cli_text("")
  parts <- build_tree_display_parts(x)

  if (parts$is_empty) {
    cli::cli_h1(parts$title)
    cli::cli_alert_warning("Empty tree - no models found")
    return(invisible(x))
  }

  cli::cli_h1(parts$title)
  cli::cli_alert_info("Models: {parts$total_models}")
  cli::cli_text("")

  final_output <- character()

  for (root_idx in seq_along(parts$root_nodes)) {
    root_node <- parts$root_nodes[root_idx]
    tree_output <- cli::tree(parts$tree_data, root = root_node)

    for (i in seq_along(tree_output)) {
      line <- tree_output[i]
      node_name <- gsub("^[^a-zA-Z0-9._]*", "", line)
      node_key <- paste0(node_name, ".mod")

      is_root <- (node_name %in% parts$root_nodes)
      children <- parts$tree_data$children[
        parts$tree_data$parent == node_name
      ][[1]]
      is_leaf <- length(children) == 0

      tree_prefix <- gsub(node_name, "", line, fixed = TRUE)
      colored_node <- if (is_root) {
        cli::col_blue(cli::style_bold(node_name))
      } else if (is_leaf) {
        cli::col_green(node_name)
      } else {
        cli::col_yellow(node_name)
      }

      if (
        node_key %in%
          names(parts$nodes) &&
          !is.null(parts$nodes[[node_key]]$description)
      ) {
        desc_text <- parts$nodes[[node_key]]$description
        if (nchar(desc_text) > 50) {
          desc_text <- paste0(substr(desc_text, 1, 47), "...")
        }
        final_output <- c(
          final_output,
          paste0(
            tree_prefix,
            colored_node,
            cli::style_dim(paste0(" - ", desc_text))
          )
        )
      } else {
        final_output <- c(final_output, paste0(tree_prefix, colored_node))
      }
    }

    if (root_idx < length(parts$root_nodes)) {
      final_output <- c(final_output, "")
    }
  }

  cat(final_output, sep = "\n")
  invisible(x)
}

#' Build Tree Data for cli::tree()
#'
#' Internal helper function to convert hyperion_nonmem_tree nodes into the exact
#' data frame format expected by cli::tree().
#'
#' @param hyperion_nonmem_tree A hyperion_nonmem_tree object
#' @return A data frame suitable for cli::tree()
#' @keywords internal
#' @noRd
build_cli_tree_data <- function(hyperion_nonmem_tree) {
  all_nodes <- names(hyperion_nonmem_tree$nodes)

  # Build children map and find unique nodes in one pass
  children_map <- list()
  unique_nodes <- all_nodes

  for (node_name in all_nodes) {
    node_info <- hyperion_nonmem_tree$nodes[[node_name]]
    if (length(node_info$based_on) > 0) {
      parent <- node_info$based_on[[1]]

      # Add any parent to unique nodes if not already present
      if (!(parent %in% unique_nodes)) {
        unique_nodes <- c(parent, unique_nodes)
      }

      # Build children map
      if (is.null(children_map[[parent]])) {
        children_map[[parent]] <- character(0)
      }
      children_map[[parent]] <- c(children_map[[parent]], node_name)
    }
  }

  # Ensure all nodes have entries in children_map
  for (node in unique_nodes) {
    if (is.null(children_map[[node]])) {
      children_map[[node]] <- character(0)
    }
  }

  # Create result data frame
  data.frame(
    stringsAsFactors = FALSE,
    parent = gsub("\\.mod$", "", unique_nodes),
    children = I(lapply(unique_nodes, function(node) {
      gsub("\\.mod$", "", children_map[[node]])
    }))
  )
}

#' Knit print method for hyperion_nonmem_tree objects (for Quarto/R Markdown)
#' @param x A hyperion_nonmem_tree object
#' @param ... Additional arguments (ignored)
#' @return HTML/markdown output for rendered documents
#' @exportS3Method knitr::knit_print
knit_print.hyperion_nonmem_tree <- function(x, ...) {
  parts <- build_tree_display_parts(x)
  output <- character()

  if (parts$is_empty) {
    output <- c(
      output,
      "",
      paste0("<strong>", parts$title, "</strong>"),
      ""
    )
    output <- c(output, "\u26a0\ufe0f Empty tree - no models found", "")
    return(knitr::asis_output(paste(output, collapse = "\n")))
  }

  output <- c(
    output,
    "",
    paste0("<strong>", parts$title, "</strong>"),
    ""
  )
  output <- c(
    output,
    paste0("\u2139\ufe0f <strong>Models:</strong> ", parts$total_models),
    ""
  )

  for (root_idx in seq_along(parts$root_nodes)) {
    root_node <- parts$root_nodes[root_idx]
    tree_lines <- knit_print_tree_node(
      root_node,
      parts$tree_data,
      parts$nodes,
      level = 0
    )
    output <- c(output, tree_lines)

    if (root_idx < length(parts$root_nodes)) {
      output <- c(output, "")
    }
  }

  knitr::asis_output(paste(output, collapse = "\n"))
}

#' Helper function to recursively build tree structure in markdown
#' @param node_name Current node name
#' @param tree_data Tree data structure from build_cli_tree_data
#' @param nodes_info Original nodes information with descriptions
#' @param level Current indentation level
#' @return Character vector of markdown lines for this subtree
#' @keywords internal
#' @noRd
knit_print_tree_node <- function(node_name, tree_data, nodes_info, level = 0) {
  output <- character()

  # Create indentation
  indent <- paste(rep("  ", level), collapse = "")

  # Find node info
  node_key <- paste0(node_name, ".mod")

  # Determine node type for styling
  all_parents <- tree_data$parent
  all_children <- unlist(tree_data$children)
  root_nodes <- setdiff(all_parents, all_children)

  is_root <- (node_name %in% root_nodes)
  children <- tree_data$children[tree_data$parent == node_name][[1]]
  is_leaf <- length(children) == 0

  # Apply HTML styling based on node type
  styled_node <- if (is_root) {
    paste0('<strong style="color:blue">', node_name, '</strong>')
  } else if (is_leaf) {
    paste0('<span style="color:green">', node_name, '</span>')
  } else {
    paste0('<span style="color:orange">', node_name, '</span>')
  }

  # Add description if available
  if (
    node_key %in%
      names(nodes_info) &&
      !is.null(nodes_info[[node_key]]$description)
  ) {
    desc_text <- nodes_info[[node_key]]$description
    if (nchar(desc_text) > 50) {
      desc_text <- paste0(substr(desc_text, 1, 47), "...")
    }
    node_line <- paste0(
      indent,
      "- ",
      styled_node,
      ' <span style="color:gray">- ',
      desc_text,
      '</span>'
    )
  } else {
    node_line <- paste0(indent, "- ", styled_node)
  }

  output <- c(output, node_line)

  # Recursively add children
  if (length(children) > 0) {
    for (child in children) {
      child_lines <- knit_print_tree_node(
        child,
        tree_data,
        nodes_info,
        level + 1
      )
      output <- c(output, child_lines)
    }
  }

  return(output)
}

# ==============================================================================
# Lineage utility functions
# ==============================================================================

#' Normalize model names with or without .mod suffix
#'
#' @param model_name Character model name
#' @param keep_suffix Logical, if TRUE preserves existing suffix or adds .mod
#' @return Normalized model name
#' @noRd
normalize_model_name <- function(model_name, keep_suffix = FALSE) {
  suffix <- NULL
  if (grepl("\\.mod$", model_name)) {
    suffix <- ".mod"
  } else if (grepl("\\.ctl$", model_name)) {
    suffix <- ".ctl"
  }
  clean <- sub("\\.(mod|ctl)$", "", model_name)
  if (keep_suffix) {
    return(paste0(clean, suffix %||% ".mod"))
  }
  clean
}

#' Get a model's ancestors
#'
#' Walk up the based_on chain to find all ancestors of a model.
#'
#' @param lineage A hyperion_nonmem_tree object from `get_model_lineage()`
#' @param model_name Character, model name (e.g., "run001" or "run001.mod")
#' @return Character vector of ancestor names (without .mod suffix),
#'   ordered from parent to root. Returns empty vector if no ancestors.
#' @export
get_model_ancestors <- function(lineage, model_name) {
  if (!inherits(lineage, "hyperion_nonmem_tree")) {
    rlang::abort("lineage must be a hyperion_nonmem_tree object")
  }

  # Normalize model name (add .mod if needed)
  model_key <- normalize_model_name(model_name, keep_suffix = TRUE)

  ancestors <- character(0)
  current <- model_key
  visited <- character(0)

  # Walk up the based_on chain

  while (TRUE) {
    if (current %in% visited) {
      rlang::abort(sprintf("Circular lineage detected at %s", current))
    }
    visited <- c(visited, current)
    node <- lineage$nodes[[current]]
    if (is.null(node) || length(node$based_on) == 0) {
      break
    }
    parent <- node$based_on[[1]]
    # Normalize parent name
    parent_clean <- normalize_model_name(parent)
    ancestors <- c(ancestors, parent_clean)
    current <- normalize_model_name(parent, keep_suffix = TRUE)
  }

  ancestors
}

#' Get a model's descendants
#'
#' Find all models whose based_on chain includes the given model.
#'
#' @param lineage A hyperion_nonmem_tree object from `get_model_lineage()`
#' @param model_name Character, model name (e.g., "run001" or "run001.mod")
#' @return Character vector of descendant names (without .mod suffix)
#' @export
get_model_descendants <- function(lineage, model_name) {
  if (!inherits(lineage, "hyperion_nonmem_tree")) {
    rlang::abort("lineage must be a hyperion_nonmem_tree object")
  }

  # Normalize model name (remove .mod if present)
  model_clean <- normalize_model_name(model_name)

  descendants <- character(0)

  # Build parent -> children map once
  parent_map <- list()
  for (node_name in names(lineage$nodes)) {
    node <- lineage$nodes[[node_name]]
    if (!is.null(node) && length(node$based_on) > 0) {
      parent_clean <- normalize_model_name(node$based_on[[1]])
      child_clean <- normalize_model_name(node_name)
      parent_map[[parent_clean]] <- unique(c(
        parent_map[[parent_clean]],
        child_clean
      ))
    }
  }

  # Traverse descendants from the starting model
  queue <- model_clean
  visited <- character(0)

  while (length(queue) > 0) {
    current <- queue[[1]]
    queue <- queue[-1]
    children <- parent_map[[current]]
    if (length(children) == 0) {
      next
    }
    for (child in children) {
      if (child %in% visited) {
        next
      }
      visited <- c(visited, child)
      descendants <- c(descendants, child)
      queue <- c(queue, child)
    }
  }

  descendants
}

#' Check if two models are in a direct lineage
#'
#' Returns TRUE if model1 is an ancestor of model2 or vice versa
#' (i.e., they are in a direct parent-child chain).
#'
#' @param lineage A hyperion_nonmem_tree object from `get_model_lineage()`
#' @param model1 Character, model name (e.g., "run001" or "run001.mod")
#' @param model2 Character, model name (e.g., "run003" or "run003.mod")
#' @return Logical, TRUE if models are in direct lineage
#' @export
are_models_in_lineage <- function(lineage, model1, model2) {
  if (!inherits(lineage, "hyperion_nonmem_tree")) {
    rlang::abort("lineage must be a hyperion_nonmem_tree object")
  }

  # Normalize model names
  model1_clean <- normalize_model_name(model1)
  model2_clean <- normalize_model_name(model2)

  # Check if model1 is ancestor of model2
  ancestors2 <- get_model_ancestors(lineage, model2)
  if (model1_clean %in% ancestors2) {
    return(TRUE)
  }

  # Check if model2 is ancestor of model1
  ancestors1 <- get_model_ancestors(lineage, model1)
  if (model2_clean %in% ancestors1) {
    return(TRUE)
  }

  FALSE
}
