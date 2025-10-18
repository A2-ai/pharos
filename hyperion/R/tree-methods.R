#' Plot Hyperion Tree Visualization
#'
#' Creates a visualization directly from a hyperion_tree object.
#' This function combines graph creation and plotting into a single step
#' for a cleaner interface.
#'
#' @param hyperion_tree An S3 object of class "hyperion_tree" containing a 'nodes' element,
#'   where each node has 'description' and optionally 'based_on' elements indicating parent relationships
#' @param layout Character string specifying the layout type.
#'   Options include "tree" (default), "sugiyama", "nicely", "kk", or "circular"
#'
#' @return A ggplot object containing the model tree visualization
#'
#' @examples
#' \dontrun{
#' # Plot hyperion_tree directly
#' plot_hyperion_tree(my_hyperion_tree)
#' plot_hyperion_tree(my_hyperion_tree, layout = "circular")
#' }
#'
#' @importFrom rlang .data
#'
#' @export
plot_hyperion_tree <- function(hyperion_tree, layout = "tree") {
  # Check for required packages
  if (!requireNamespace("igraph", quietly = TRUE)) {
    stop("Package 'igraph' is required for plotting. Please install it with: install.packages('igraph')")
  }
  if (!requireNamespace("ggraph", quietly = TRUE)) {
    stop("Package 'ggraph' is required for plotting. Please install it with: install.packages('ggraph')")
  }
  if (!requireNamespace("ggplot2", quietly = TRUE)) {
    stop("Package 'ggplot2' is required for plotting. Please install it with: install.packages('ggplot2')")
  }

  # Parameter validation
  if (!inherits(hyperion_tree, "hyperion_tree")) {
    stop("hyperion_tree must be an S3 object of class 'hyperion_tree'")
  }

  if (is.null(hyperion_tree$nodes) || !is.list(hyperion_tree$nodes)) {
    stop("hyperion_tree must contain a 'nodes' element that is a list")
  }

  if (length(hyperion_tree$nodes) == 0) {
    warning("hyperion_tree contains no nodes")
    return(ggplot2::ggplot() +
      ggplot2::theme_void())
  }

  if (!is.character(layout) || length(layout) != 1) {
    stop("layout must be a single character string")
  }

  # Extract relationships from the tree structure
  edges_list <- list()
  nodes_list <- list()

  # Process each node in the tree
  for (model_name in names(hyperion_tree$nodes)) {
    node_info <- hyperion_tree$nodes[[model_name]]

    # Add node information
    nodes_list[[model_name]] <- list(
      name = model_name,
      description = if (is.null(node_info$description)) "" else node_info$description
    )

    # Add edge information (parent -> child relationship)
    if (length(node_info$based_on) > 0) {
      parent <- node_info$based_on[[1]]
      # Clean up model names - remove .mod extension for parents if needed
      if (!grepl("\\.mod$", parent)) {
        parent <- paste0(parent, ".mod")
      }

      edges_list[[paste(parent, model_name, sep = "_")]] <- list(
        from = parent,
        to = model_name
      )
    }
  }

  # Convert to data frames
  edges <- do.call(rbind, lapply(edges_list, data.frame, stringsAsFactors = FALSE))
  nodes_info <- do.call(rbind, lapply(nodes_list, data.frame, stringsAsFactors = FALSE))

  # Handle the root node "nonmem"
  if (any(edges$from == "nonmem.mod")) {
    nodes_info <- rbind(nodes_info, data.frame(
      name = "nonmem.mod",
      description = "Base NONMEM model",
      stringsAsFactors = FALSE
    ))
  }

  # Create igraph object
  g <- igraph::graph_from_data_frame(edges, directed = TRUE, vertices = nodes_info)

  # Set up layout
  layout_args <- list(g = g, layout = layout)
  if (layout == "tree") {
    layout_args$root <- "nonmem.mod"
  } else if (layout == "circular") {
    layout_args$layout <- "dendrogram"
    layout_args$circular <- TRUE
  }

  graph_layout <- do.call(ggraph::create_layout, layout_args)

  # Dynamic sizing based on number of nodes
  n_nodes <- igraph::vcount(g)
  size_params <- if (n_nodes > 50) {
    list(node_size = 2, text_size = 2.5, edge_width = 0.3, cap_size = 2, nudge_dist = -0.2)
  } else if (n_nodes > 20) {
    list(node_size = 3, text_size = 3, edge_width = 0.4, cap_size = 2.5, nudge_dist = -0.25)
  } else {
    list(node_size = 4, text_size = 3.2, edge_width = 0.5, cap_size = 3, nudge_dist = -0.3)
  }

  # Create the plot with adaptive styling
  p <- ggraph::ggraph(graph_layout) +
    ggraph::geom_edge_link(
      arrow = ggplot2::arrow(length = ggplot2::unit(1.5, "mm")),
      color = "gray60",
      width = size_params$edge_width,
      start_cap = ggraph::circle(size_params$cap_size, "mm"),
      end_cap = ggraph::circle(size_params$cap_size, "mm")
    ) +
    ggraph::geom_node_point(size = size_params$node_size, color = "steelblue", alpha = 0.8) +
    ggraph::geom_node_text(ggplot2::aes(label = gsub("\\.mod$", "", .data$name)),
      nudge_y = size_params$nudge_dist,
      size = size_params$text_size,
      color = "black"
    ) +
    ggraph::theme_graph()

  return(p)
}

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
      is_intermediate <- !is_root && !is_leaf

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

  # Footer message
  cli::cli_text("")
  cli::cli_text("{.emph Use} {.code plot_hyperion_tree()} {.emph for visualization}")

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
