#' Print Method for Hyperion Tree Objects
#'
#' Displays a hyperion_tree in a readable tree format using cli::tree().
#' Shows the hierarchical relationships between models with Unicode tree characters.
#'
#' @param x A hyperion_tree object
#' @param ... Additional arguments (currently unused)
#'
#' @return Invisibly returns the input object
#' @export
print.hyperion_tree <- function(x, ...) {
  # Handle empty tree
  if (is.null(x$nodes) || length(x$nodes) == 0) {
    cli::cli_h1("Hyperion Model Tree")
    cli::cli_alert_warning("Empty tree - no models found")
    return(invisible(x))
  }

  # Header with model count
  cli::cli_h1("Hyperion Model Tree")
  cli::cli_alert_info("Models: {length(x$nodes)}")
  cli::cli_text("")

  # Build data structure and use cli::tree with descriptions
  tree_data <- build_cli_tree_data(x)

  # Find root nodes (nodes that have no parents in the tree)
  all_packages <- tree_data$package
  all_children <- unlist(tree_data$dependencies)
  root_nodes <- setdiff(all_packages, all_children)

  # Display each root as a separate tree
  final_output <- character()

  for (root_idx in seq_along(root_nodes)) {
    root_node <- root_nodes[root_idx]

    # Generate tree for this root
    tree_output <- cli::tree(tree_data, root = root_node)

    # Process each line of this tree
    for (i in seq_along(tree_output)) {
      line <- tree_output[i]
      # Extract node name from the line (after tree characters)
      node_name <- gsub("^[^a-zA-Z0-9._]*", "", line) # Remove leading tree chars
      node_key <- paste0(node_name, ".mod")

      # Determine node type for coloring
      is_root <- (node_name %in% root_nodes)
      children <- tree_data$dependencies[tree_data$package == node_name][[1]]
      is_leaf <- length(children) == 0

      # Apply colors to node name
      tree_prefix <- gsub(node_name, "", line, fixed = TRUE) # Get tree characters
      colored_node <- if (is_root) {
        cli::col_blue(cli::style_bold(node_name))
      } else if (is_leaf) {
        cli::col_green(node_name)
      } else {
        cli::col_yellow(node_name)
      }

      # Add description if available
      if (node_key %in% names(x$nodes) && !is.null(x$nodes[[node_key]]$description)) {
        desc_text <- x$nodes[[node_key]]$description
        if (nchar(desc_text) > 50) {
          desc_text <- paste0(substr(desc_text, 1, 47), "...")
        }
        final_output <- c(final_output, paste0(tree_prefix, colored_node, cli::style_dim(paste0(" - ", desc_text))))
      } else {
        final_output <- c(final_output, paste0(tree_prefix, colored_node))
      }
    }

    # Add blank line between trees (except after the last one)
    if (root_idx < length(root_nodes)) {
      final_output <- c(final_output, "")
    }
  }

  # Print the enhanced tree(s)
  cat(final_output, sep = "\n")
  invisible(x)
}

#' Build Tree Data for cli::tree()
#'
#' Internal helper function to convert hyperion_tree nodes into the exact
#' data frame format expected by cli::tree().
#'
#' @param hyperion_tree A hyperion_tree object
#' @return A data frame suitable for cli::tree()
#' @keywords internal
#' @noRd
build_cli_tree_data <- function(hyperion_tree) {
  all_nodes <- names(hyperion_tree$nodes)

  # Build children map and find unique nodes in one pass
  children_map <- list()
  unique_nodes <- all_nodes

  for (node_name in all_nodes) {
    node_info <- hyperion_tree$nodes[[node_name]]
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
    package = gsub("\\.mod$", "", unique_nodes),
    dependencies = I(lapply(unique_nodes, function(node) {
      gsub("\\.mod$", "", children_map[[node]])
    }))
  )
}
