#' NONMEM Model class
#'
#' A comprehensive model object that provides unified access to model files,
#' run outputs, parameter information, and lineage tracking. Properties are
#' computed on access, with `info` supporting modification and caching.
#'
#' @param model_path Character. Path to the NONMEM model file (.mod or .ctl).
#' @param lookup_path Character or NULL. Optional path to a TOML lookup file
#'   for additional parameter metadata.
#'
#' @section Properties:
#' \describe{
#'   \item{model_path}{Path to the NONMEM model file.}
#'   \item{lookup_path}{Optional path to TOML lookup file for parameter metadata.}
#'   \item{model_name}{Model name derived from filename (without extension).
#'     Computed from `model_path`.}
#'   \item{model}{The parsed `hyperion_nonmem_model` object. Equivalent to
#'     calling `read_model(model_path)`.}
#'   \item{summary}{Model run summary as `hyperion_nonmem_summary`. Equivalent
#'     to calling `get_model_summary(model)`.}
#'   \item{info}{Parameter comments as `ModelComments`. Supports modification
#'     via direct assignment (e.g., `model@info@theta$THETA1@name <- "CL"`).
#'     Changes persist until the object is discarded.}
#'   \item{info_audit}{Audit results for parameter info completeness.}
#'   \item{lineage}{Model lineage tree as `hyperion_nonmem_tree`.}
#'   \item{ancestors}{Character vector of ancestor model names.}
#'   \item{descendants}{Character vector of descendant model names.}
#'   \item{parameters}{Data frame of parameter estimates from the model run.}
#'   \item{run_info}{List containing run details and heuristics.}
#'   \item{eta_shrinkage}{Data frame of ETA shrinkage metrics.}
#'   \item{eps_shrinkage}{Data frame of EPS shrinkage metrics.}
#'   \item{gradients}{Data frame of parameter gradients during estimation.}
#' }
#'
#' @examples
#' \dontrun{
#' # Create a Model object
#' model <- Model("path/to/run001.mod")
#'
#' # Access properties
#' model@model_name
#' model@parameters
#' model@summary
#'
#' # Modify parameter info
#' model@info@theta$THETA1@display <- "Clearance"
#' model@info@omega$`OMEGA(1,1)`@parameterization <- "LogNormal"
#' }
#'
#' @include comments-classes.R
#' @export
Model <- S7::new_class(
  "Model",
  properties = list(
    model_path = S7::class_character,
    lookup_path = S7::class_character | NULL,
    .info = S7::new_property(ModelComments | NULL, default = NULL),
    model_name = S7::new_property(
      class = S7::class_character,
      getter = function(self) {
        self@model_path |>
          basename() |>
          tools::file_path_sans_ext()
      }
    ),
    model = S7::new_property(
      S7::new_S3_class("hyperion_nonmem_model"),
      getter = function(self) {
        read_model(self@model_path)
      }
    ),
    summary = S7::new_property(
      S7::new_S3_class("hyperion_nonmem_summary"),
      getter = function(self) {
        get_model_summary(self@model)
      }
    ),
    info = S7::new_property(
      ModelComments,
      getter = function(self) {
        if (!is.null(self@.info)) {
          return(self@.info)
        }
        get_model_parameter_info(self@model, self@lookup_path)
      },
      setter = function(self, value) {
        self@.info <- value
        self
      }
    ),
    info_audit = S7::new_property(
      S7::new_S3_class("parameter_audit"),
      getter = function(self) {
        audit_parameter_info(self@info)
      }
    ),
    lineage = S7::new_property(
      S7::new_S3_class("hyperion_nonmem_tree"),
      getter = function(self) {
        get_model_lineage(self@model)
      }
    ),
    ancestors = S7::new_property(
      class = S7::class_character,
      getter = function(self) {
        get_model_ancestors(self@lineage, self@model_name)
      }
    ),
    descendants = S7::new_property(
      class = S7::class_character,
      getter = function(self) {
        get_model_descendants(self@lineage, self@model_name)
      }
    ),
    parameters = S7::new_property(
      class = S7::class_data.frame,
      getter = function(self) {
        get_parameters(self@model)
      }
    ),
    run_info = S7::new_property(
      class = S7::class_list,
      getter = function(self) {
        get_run_info(self@model)
      }
    ),
    eta_shrinkage = S7::new_property(
      class = S7::class_data.frame,
      getter = function(self) {
        get_eta_shrinkage(self@model)
      }
    ),
    eps_shrinkage = S7::new_property(
      class = S7::class_data.frame,
      getter = function(self) {
        get_eps_shrinkage(self@model)
      }
    ),
    gradients = S7::new_property(
      class = S7::class_data.frame,
      getter = function(self) {
        get_gradients(self@model)
      }
    )
  ),
  validator = function(self) {
    if (!is.null(self@lookup_path)) {
      if (self@lookup_path |> tools::file_ext() != "toml") {
        sprintf(
          "@lookup_path must be a toml file, got: (%s)",
          self@lookup_path |> tools::file_ext()
        )
      }
    }
  },
  constructor = function(model_path, lookup_path = NULL) {
    S7::new_object(
      S7::S7_object(),
      model_path = model_path,
      lookup_path = lookup_path
    )
  }
)

# run003 <- Model("inst/extdata/models/onecmt/run003.mod")
