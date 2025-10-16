
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
    return(ggplot2::ggplot() + ggplot2::theme_void())
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
    ggraph::geom_edge_link(arrow = ggplot2::arrow(length = ggplot2::unit(1.5, "mm")),
                           color = "gray60",
                           width = size_params$edge_width,
                           start_cap = ggraph::circle(size_params$cap_size, 'mm'),
                           end_cap = ggraph::circle(size_params$cap_size, 'mm')) +
    ggraph::geom_node_point(size = size_params$node_size, color = "steelblue", alpha = 0.8) +
    ggraph::geom_node_text(ggplot2::aes(label = gsub("\\.mod$", "", name)),
                           nudge_y = size_params$nudge_dist,
                           size = size_params$text_size,
                           color = "black") +
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

  # Build data structure and display styled tree
  tree_data <- build_cli_tree_data(x)
  display_styled_tree(x, tree_data)

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

      # Add nonmem to unique nodes if referenced
      if (parent == "nonmem" && !("nonmem" %in% unique_nodes)) {
        unique_nodes <- c("nonmem", unique_nodes)
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

#' Display Styled Tree
#'
#' Internal helper to display a styled tree with colors and descriptions
#'
#' @param hyperion_tree Original hyperion_tree object for descriptions
#' @param tree_data Data frame returned by build_cli_tree_data
#' @keywords internal
#' @noRd
display_styled_tree <- function(hyperion_tree, tree_data) {

  # Build children map from tree_data
  children_map <- stats::setNames(tree_data$dependencies, tree_data$package)

  # Find actual root nodes (models based on nonmem, excluding nonmem itself)
  all_children <- unlist(tree_data$dependencies)
  potential_roots <- tree_data$package[!tree_data$package %in% all_children]
  roots <- potential_roots[potential_roots != "nonmem"]

  # If nonmem is a root, its children become the roots instead
  if ("nonmem" %in% potential_roots) {
    nonmem_children <- children_map[["nonmem"]]
    roots <- c(roots, nonmem_children)
  }

  # Print each root and its subtree
  for (i in seq_along(roots)) {
    print_tree_node(roots[i], hyperion_tree, children_map, "", i == length(roots))
  }
}

#' Print Tree Node with Styling
#'
#' Recursively prints a tree node and its children with colors and descriptions
#'
#' @param node_name Name of the node to print
#' @param hyperion_tree Original hyperion_tree object
#' @param children_map Named list mapping node names to their children
#' @param prefix String prefix for indentation
#' @param is_last Whether this is the last child at this level
#' @keywords internal
#' @noRd
print_tree_node <- function(node_name, hyperion_tree, children_map, prefix = "", is_last = TRUE) {

  # Get node info for description
  node_key <- paste0(node_name, ".mod")
  node_info <- hyperion_tree$nodes[[node_key]]

  description <- ""
  is_based_on_nonmem <- FALSE

  if (!is.null(node_info)) {
    # Check if this node is based on nonmem
    is_based_on_nonmem <- (length(node_info$based_on) > 0 && node_info$based_on[[1]] == "nonmem")

    # Handle description with truncation
    desc_text <- node_info$description
    if (!is.null(desc_text) && nchar(desc_text) > 0) {
      description <- if (nchar(desc_text) > 50) {
        paste0(substr(desc_text, 1, 47), "...")
      } else {
        desc_text
      }
    }
  }

  # Determine node type and color
  children <- children_map[[node_name]]
  has_children <- length(children) > 0

  # Style node name based on type
  styled_name <- if (is_based_on_nonmem && prefix == "") {
    # Root model (based on nonmem)
    cli::col_blue(cli::style_bold(node_name))
  } else if (!has_children) {
    # Leaf node
    cli::col_green(node_name)
  } else {
    # Intermediate node
    cli::col_yellow(node_name)
  }

  # Style description
  styled_desc <- if (description != "") {
    cli::style_dim(paste0(" - ", description))
  } else {
    ""
  }

  # Print current node with proper tree characters
  if (prefix == "") {
    # Root level
    cli::cli_text(paste0(styled_name, styled_desc))
  } else {
    # Child level
    connector <- if (is_last) "└─" else "├─"
    cli::cli_text(paste0(prefix, connector, styled_name, styled_desc))
  }

  # Print children
  if (has_children) {
    for (i in seq_along(children)) {
      child <- children[i]
      is_last_child <- (i == length(children))
      new_prefix <- if (prefix == "") {
        # Root level children get basic prefix for tree structure
        if (is_last_child) "  " else "│ "
      } else {
        # Deeper levels - add appropriate continuation
        paste0(prefix, if (is_last) "  " else "│ ")
      }
      print_tree_node(child, hyperion_tree, children_map, new_prefix, is_last_child)
    }
  }
}
