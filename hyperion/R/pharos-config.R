#' Set comment type to type1 in pharos.toml
#'
#' Modifies the pharos.toml configuration file to use type1 comment parsing.
#' This is useful when NONMEM control streams use the type1 comment format.
#'
#' @param path Path to pharos.toml. If NULL, finds it automatically.
#' @return The path to the modified pharos.toml file (invisibly).
#' @export
#'
#' @examples \dontrun{
#' use_type1_comments()
#' }
use_type1_comments <- function(path = NULL) {
  if (is.null(path)) {
    path <- find_pharos_config_file()
    if (grepl("No pharos.toml", path)) {
      stop("pharos.toml not found. Run init() first.")
    }
  }

  toml <- tomledit::read_toml(path)
  nonmem <- tomledit::get_item(toml, "nonmem")
  nonmem$comments$type <- "type1"
  toml <- tomledit::insert_items(toml, nonmem = nonmem)
  tomledit::write_toml(toml, path)

  invisible(path)
}
