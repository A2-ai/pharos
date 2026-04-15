#!/usr/bin/env Rscript

`%||%` <- function(x, y) if (is.null(x) || length(x) == 0 || !nzchar(x[[1]])) y else x

script_path <- function() {
  file_arg <- grep("^--file=", commandArgs(FALSE), value = TRUE)
  if (length(file_arg) == 0) {
    stop("Unable to determine script path.")
  }
  normalizePath(sub("^--file=", "", file_arg[[1]]), winslash = "/", mustWork = TRUE)
}

script_dir <- dirname(script_path())

normalize_existing <- function(path) {
  normalizePath(path, winslash = "/", mustWork = TRUE)
}

normalize_maybe <- function(path) {
  normalizePath(path, winslash = "/", mustWork = FALSE)
}

trim_trailing_slash <- function(path) {
  sub("[/\\\\]+$", "", path)
}

is_under_path <- function(path, root) {
  path <- trim_trailing_slash(normalize_maybe(path))
  root <- trim_trailing_slash(normalize_maybe(root))
  identical(path, root) || startsWith(path, paste0(root, "/"))
}

with_dir <- function(path, expr) {
  old <- setwd(path)
  on.exit(setwd(old), add = TRUE)
  force(expr)
}

with_env <- function(env, expr) {
  if (length(env) == 0) {
    return(force(expr))
  }

  old <- Sys.getenv(names(env), unset = NA_character_)
  names(old) <- names(env)
  do.call(Sys.setenv, as.list(env))
  on.exit({
    missing <- names(old)[is.na(old)]
    present <- old[!is.na(old)]
    if (length(missing) > 0) {
      Sys.unsetenv(missing)
    }
    if (length(present) > 0) {
      do.call(Sys.setenv, as.list(present))
    }
  }, add = TRUE)

  force(expr)
}

run_command <- function(command,
                        args = character(),
                        wd = NULL,
                        env = character(),
                        allow_error = FALSE) {
  run_once <- function() {
    system2(command, args = args, stdout = TRUE, stderr = TRUE)
  }

  output <- if (!is.null(wd)) {
    with_dir(wd, with_env(env, run_once()))
  } else {
    with_env(env, run_once())
  }

  status <- attr(output, "status")
  if (is.null(status)) {
    status <- 0L
  }

  if (!allow_error && status != 0L) {
    stop(
      paste(
        c(
          sprintf("Command failed: %s %s", command, paste(args, collapse = " ")),
          output
        ),
        collapse = "\n"
      ),
      call. = FALSE
    )
  }

  list(
    status = status,
    success = identical(status, 0L),
    output = output
  )
}

regex_escape <- function(x) {
  gsub("([][{}()+*^$.|\\\\?])", "\\\\\\1", x)
}

parse_tree_packages <- function(lines) {
  parsed <- lapply(lines, function(line) {
    line <- trimws(line)
    if (!nzchar(line)) {
      return(NULL)
    }

    tokens <- strsplit(line, "\\s+")[[1]]
    if (length(tokens) < 2 || !startsWith(tokens[[2]], "v")) {
      return(NULL)
    }

    path <- NA_character_
    if (grepl(" \\([^()]*[/\\\\][^()]*\\)$", line)) {
      path <- sub("^.* \\(([^()]*[/\\\\][^()]*)\\)$", "\\1", line)
      if (grepl("^https?://", path) || grepl("^git[+@]", path)) {
        path <- NA_character_
      }
    }

    data.frame(
      name = tokens[[1]],
      version = sub("^v", "", tokens[[2]]),
      path = path,
      stringsAsFactors = FALSE
    )
  })

  parsed <- Filter(Negate(is.null), parsed)
  if (length(parsed) == 0) {
    return(data.frame(name = character(), version = character(), path = character(), stringsAsFactors = FALSE))
  }

  result <- do.call(rbind, parsed)
  result[!duplicated(result), , drop = FALSE]
}

copy_tree <- function(from, to) {
  dir.create(to, recursive = TRUE, showWarnings = FALSE)
  contents <- list.files(from, all.files = TRUE, no.. = TRUE, full.names = TRUE)
  if (length(contents) == 0) {
    return(invisible(TRUE))
  }

  ok <- file.copy(contents, to, recursive = TRUE)
  if (!all(ok)) {
    stop(sprintf("Failed to copy %s into %s", from, to), call. = FALSE)
  }

  invisible(TRUE)
}

cargo_tree_packages <- function(manifest_path, depth = NULL) {
  args <- c(
    "tree",
    "-q",
    "--manifest-path", manifest_path,
    "--prefix", "none",
    "--charset", "ascii",
    "--edges", "normal,build,dev",
    "--format", "{p}"
  )
  if (!is.null(depth)) {
    args <- c(args, "--depth", as.character(depth))
  }
  result <- run_command("cargo", args)
  parse_tree_packages(result$output)
}

scan_manifest_dependency_names <- function(manifest_path) {
  lines <- readLines(manifest_path, warn = FALSE)
  names <- character()
  in_section <- FALSE

  for (line in trimws(lines)) {
    if (!nzchar(line) || startsWith(line, "#")) {
      next
    }

    if (startsWith(line, "[")) {
      section <- sub("^\\[(.*)\\]$", "\\1", line)
      in_section <- grepl("(^|\\.)((build|dev)\\-)?dependencies$", section) &&
        !grepl("^patch\\.", section)
      next
    }

    if (!in_section) {
      next
    }

    match <- regexec("^([A-Za-z0-9_.-]+)\\s*=", line)
    value <- regmatches(line, match)[[1]]
    if (length(value) == 2) {
      names <- c(names, value[[2]])
    }
  }

  unique(names)
}

root_dependency_names <- function(manifest_path) {
  tree <- tryCatch(
    cargo_tree_packages(manifest_path),
    error = function(e) NULL
  )

  if (!is.null(tree) && nrow(tree) > 0) {
    return(unique(tree$name))
  }

  scan_manifest_dependency_names(manifest_path)
}

scan_source_manifests <- function(source_root) {
  manifests <- list.files(
    source_root,
    pattern = "^Cargo\\.toml$",
    recursive = TRUE,
    full.names = TRUE,
    include.dirs = FALSE
  )

  manifests <- manifests[!grepl("(^|/)(target|vendor|\\.git|\\.cargo)(/|$)", manifests)]
  unique(normalizePath(manifests, winslash = "/", mustWork = FALSE))
}

scan_source_packages <- function(source_root) {
  manifests <- scan_source_manifests(source_root)
  if (length(manifests) == 0) {
    return(data.frame(
      name = character(),
      version = character(),
      manifest_path = character(),
      crate_dir = character(),
      stringsAsFactors = FALSE
    ))
  }

  packages <- lapply(manifests, function(manifest_path) {
    root_only <- tryCatch(
      suppressWarnings(cargo_tree_packages(manifest_path, depth = 0)),
      error = function(e) NULL
    )

    if (is.null(root_only) || nrow(root_only) == 0) {
      return(NULL)
    }

    pkg <- root_only[1, , drop = FALSE]
    if (is.na(pkg$path[[1]]) || !nzchar(pkg$path[[1]])) {
      return(NULL)
    }

    data.frame(
      name = pkg$name[[1]],
      version = pkg$version[[1]],
      manifest_path = normalizePath(manifest_path, winslash = "/", mustWork = FALSE),
      crate_dir = normalizePath(dirname(manifest_path), winslash = "/", mustWork = FALSE),
      stringsAsFactors = FALSE
    )
  })

  packages <- Filter(Negate(is.null), packages)
  if (length(packages) == 0) {
    return(data.frame(
      name = character(),
      version = character(),
      manifest_path = character(),
      crate_dir = character(),
      stringsAsFactors = FALSE
    ))
  }

  result <- do.call(rbind, packages)
  result[!duplicated(result[, c("name", "manifest_path")]), , drop = FALSE]
}

auto_source_root <- function(package_root) {
  env_root <- Sys.getenv("MINIEXTENDR_LOCAL", unset = "")
  if (nzchar(env_root) && dir.exists(env_root)) {
    return(normalize_existing(env_root))
  }

  recorded_source <- file.path(package_root, "vendor", ".vendor-source")
  if (file.exists(recorded_source)) {
    recorded <- trimws(readLines(recorded_source, warn = FALSE, n = 1))
    if (nzchar(recorded) && dir.exists(recorded)) {
      return(normalize_existing(recorded))
    }
  }

  probe <- normalize_existing(package_root)
  repeat {
    parent <- dirname(probe)
    if (identical(parent, probe)) {
      break
    }
    if (file.exists(file.path(parent, "Cargo.toml"))) {
      return(normalize_existing(parent))
    }
    probe <- parent
  }

  NULL
}

discover_local_roots <- function(manifest_path, package_root, source_root = NULL) {
  package_rust_dir <- file.path(package_root, "src", "rust")
  package_vendor_dir <- file.path(package_root, "vendor")

  current_tree <- tryCatch(
    cargo_tree_packages(manifest_path),
    error = function(e) NULL
  )

  if (!is.null(current_tree) && nrow(current_tree) > 0) {
    local <- current_tree[!is.na(current_tree$path) & nzchar(current_tree$path), , drop = FALSE]
    if (nrow(local) > 0) {
      keep <- !vapply(local$path, is_under_path, logical(1), root = package_rust_dir) &
        !vapply(local$path, is_under_path, logical(1), root = package_vendor_dir)

      if (any(keep)) {
        local <- local[keep, , drop = FALSE]
        return(data.frame(
          name = local$name,
          version = local$version,
          manifest_path = file.path(local$path, "Cargo.toml"),
          crate_dir = local$path,
          stringsAsFactors = FALSE
        )[!duplicated(local[, c("name", "path")]), , drop = FALSE])
      }
    }
  }

  if (!is.null(source_root)) {
    source_packages <- scan_source_packages(source_root)
    if (nrow(source_packages) == 0) {
      return(source_packages)
    }

    dependency_names <- root_dependency_names(manifest_path)
    keep <- source_packages$name %in% dependency_names &
      !vapply(source_packages$crate_dir, is_under_path, logical(1), root = package_rust_dir)

    result <- source_packages[keep, , drop = FALSE]
    return(result[!duplicated(result[, c("name", "manifest_path")]), , drop = FALSE])
  }

  tree <- current_tree %||% cargo_tree_packages(manifest_path)
  local <- tree[!is.na(tree$path) & nzchar(tree$path), , drop = FALSE]
  if (nrow(local) == 0) {
    return(data.frame(
      name = character(),
      version = character(),
      manifest_path = character(),
      crate_dir = character(),
      stringsAsFactors = FALSE
    ))
  }

  keep <- !vapply(local$path, is_under_path, logical(1), root = package_rust_dir) &
    !vapply(local$path, is_under_path, logical(1), root = package_vendor_dir)

  local <- local[keep, , drop = FALSE]
  if (nrow(local) == 0) {
    return(data.frame(
      name = character(),
      version = character(),
      manifest_path = character(),
      crate_dir = character(),
      stringsAsFactors = FALSE
    ))
  }

  data.frame(
    name = local$name,
    version = local$version,
    manifest_path = file.path(local$path, "Cargo.toml"),
    crate_dir = local$path,
    stringsAsFactors = FALSE
  )[!duplicated(local[, c("name", "path")]), , drop = FALSE]
}

patch_packages_for_crate <- function(crate_manifest, package_root, source_root = NULL) {
  crate_dir <- normalize_maybe(dirname(crate_manifest))
  package_rust_dir <- file.path(package_root, "src", "rust")
  package_vendor_dir <- file.path(package_root, "vendor")

  tree <- cargo_tree_packages(crate_manifest)
  local <- tree[!is.na(tree$path) & nzchar(tree$path), , drop = FALSE]
  if (nrow(local) == 0) {
    return(local)
  }

  keep <- normalize_maybe(local$path) != crate_dir
  if (!is.null(source_root)) {
    keep <- keep & vapply(local$path, is_under_path, logical(1), root = source_root)
  } else {
    keep <- keep &
      !vapply(local$path, is_under_path, logical(1), root = package_rust_dir) &
      !vapply(local$path, is_under_path, logical(1), root = package_vendor_dir)
  }

  local <- local[keep, , drop = FALSE]
  local[!duplicated(local[, c("name", "path")]), , drop = FALSE]
}

write_patch_config <- function(packages, target_dir) {
  if (nrow(packages) == 0) {
    return(NULL)
  }

  config_path <- file.path(target_dir, "cargo-package-patches.toml")
  lines <- c("[patch.crates-io]")
  for (i in seq_len(nrow(packages))) {
    lines <- c(
      lines,
      sprintf(
        '%s = { path = "%s" }',
        packages$name[[i]],
        gsub("\\\\", "/", normalize_maybe(packages$path[[i]]))
      )
    )
  }
  writeLines(lines, config_path)
  config_path
}

strip_toml_sections <- function(lines, headers) {
  trimmed <- trimws(lines)
  is_target <- trimmed %in% headers
  # Also match table subsections: [dev-dependencies.X] for header [dev-dependencies]
  for (h in headers) {
    if (!startsWith(h, "[[") && endsWith(h, "]")) {
      prefix <- paste0(substr(h, 1, nchar(h) - 1), ".")
      is_target <- is_target | startsWith(trimmed, prefix)
    }
  }
  is_any_header <- grepl("^\\[", trimmed)
  keep <- rep(TRUE, length(lines))
  in_section <- FALSE
  for (i in seq_along(lines)) {
    if (is_target[i]) {
      in_section <- TRUE
      keep[i] <- FALSE
    } else if (in_section) {
      if (is_any_header[i]) {
        in_section <- FALSE
      } else {
        keep[i] <- FALSE
      }
    }
  }
  lines[keep]
}

strip_vendored_crate <- function(crate_path) {
  unwanted_dirs <- c(
    "target", ".git", ".github", ".circleci", "tests", "benches",
    "examples", "docs", "ci"
  )

  for (entry in unwanted_dirs) {
    path <- file.path(crate_path, entry)
    if (dir.exists(path)) {
      unlink(path, recursive = TRUE, force = TRUE)
    }
  }

  top_level <- list.files(crate_path, all.files = TRUE, no.. = TRUE, full.names = TRUE)
  dotfiles <- top_level[startsWith(basename(top_level), ".")]
  dotfiles <- dotfiles[basename(dotfiles) != ".cargo-checksum.json"]
  if (length(dotfiles) > 0) {
    unlink(dotfiles, recursive = TRUE, force = TRUE)
  }

  manifest <- file.path(crate_path, "Cargo.toml")
  if (file.exists(manifest)) {
    lines <- readLines(manifest, warn = FALSE)
    lines <- strip_toml_sections(lines, c("[[bench]]", "[[test]]", "[dev-dependencies]"))
    writeLines(lines, manifest)
  }
}

extract_crate_archive <- function(crate_file, vendor_dir, name, version) {
  extract_dir <- tempfile("crate-extract-")
  dir.create(extract_dir, recursive = TRUE)
  on.exit(unlink(extract_dir, recursive = TRUE, force = TRUE), add = TRUE)

  utils::untar(crate_file, exdir = extract_dir)
  extracted <- list.dirs(extract_dir, recursive = FALSE, full.names = TRUE)
  if (length(extracted) != 1) {
    stop(sprintf("Unexpected archive layout for %s", basename(crate_file)), call. = FALSE)
  }

  destination <- file.path(vendor_dir, name)
  if (dir.exists(destination)) {
    unlink(destination, recursive = TRUE, force = TRUE)
  }
  dir.create(destination, recursive = TRUE)

  contents <- list.files(extracted[[1]], all.files = TRUE, no.. = TRUE, full.names = TRUE)
  if (length(contents) > 0) {
    copy_tree(extracted[[1]], destination)
  }

  strip_vendored_crate(destination)
  writeLines('{"files":{}}', file.path(destination, ".cargo-checksum.json"))
  invisible(destination)
}

package_local_crates <- function(local_roots, package_root, source_root, staging_root) {
  if (nrow(local_roots) == 0) {
    return(data.frame(
      name = character(),
      version = character(),
      crate_file = character(),
      stringsAsFactors = FALSE
    ))
  }

  packaged <- vector("list", nrow(local_roots))

  for (i in seq_len(nrow(local_roots))) {
    crate <- local_roots[i, , drop = FALSE]
    crate_name <- crate$name[[1]]
    crate_version <- crate$version[[1]]
    crate_manifest <- crate$manifest_path[[1]]
    crate_stage <- file.path(staging_root, crate_name)
    dir.create(crate_stage, recursive = TRUE, showWarnings = FALSE)

    patches <- patch_packages_for_crate(crate_manifest, package_root, source_root)
    config_path <- write_patch_config(patches, crate_stage)
    args <- c()
    if (!is.null(config_path)) {
      args <- c(args, "--config", config_path)
    }
    args <- c(
      args,
      "package",
      "--manifest-path", crate_manifest,
      "--allow-dirty",
      "--no-verify"
    )

    run_command(
      "cargo",
      args = args,
      wd = dirname(crate_manifest),
      env = c(CARGO_TARGET_DIR = file.path(crate_stage, "target"))
    )

    expected <- sprintf("^%s-%s\\.crate$", regex_escape(crate_name), regex_escape(crate_version))
    crate_files <- list.files(
      file.path(crate_stage, "target", "package"),
      pattern = expected,
      full.names = TRUE
    )
    if (length(crate_files) != 1) {
      stop(sprintf("Expected exactly one .crate archive for %s", crate_name), call. = FALSE)
    }

    packaged[[i]] <- data.frame(
      name = crate_name,
      version = crate_version,
      crate_file = normalize_maybe(crate_files[[1]]),
      stringsAsFactors = FALSE
    )
  }

  do.call(rbind, packaged)
}

strip_lockfile_checksums <- function(lockfile) {
  if (!file.exists(lockfile)) {
    return(invisible(FALSE))
  }

  lines <- readLines(lockfile, warn = FALSE)
  lines <- lines[!grepl("^checksum = ", lines)]
  writeLines(lines, lockfile)
  invisible(TRUE)
}

compress_vendor_tarball <- function(package_root, vendor_dir) {
  inst_dir <- file.path(package_root, "inst")
  if (!dir.exists(inst_dir)) {
    dir.create(inst_dir, recursive = TRUE)
  }

  tarball <- file.path(inst_dir, "vendor.tar.xz")
  staging <- tempfile("vendor-pack-")
  dir.create(staging, recursive = TRUE)
  on.exit(unlink(staging, recursive = TRUE, force = TRUE), add = TRUE)

  staging_vendor <- file.path(staging, "vendor")
  copy_tree(vendor_dir, staging_vendor)

  md_files <- list.files(staging_vendor, pattern = "\\.md$", recursive = TRUE, full.names = TRUE)
  for (md_file in md_files) {
    writeLines(character(), md_file)
  }

  result <- run_command(
    "tar",
    c("-cJf", tarball, "-C", staging, "vendor")
  )

  if (!result$success) {
    stop("Failed to create inst/vendor.tar.xz", call. = FALSE)
  }

  normalize_maybe(tarball)
}

swap_vendor_directory <- function(staging_vendor, vendor_dir) {
  backup_dir <- tempfile("vendor-backup-")
  if (dir.exists(vendor_dir)) {
    if (!file.rename(vendor_dir, backup_dir)) {
      unlink(vendor_dir, recursive = TRUE, force = TRUE)
    }
  }
  on.exit(unlink(backup_dir, recursive = TRUE, force = TRUE), add = TRUE)

    if (!file.rename(staging_vendor, vendor_dir)) {
    unlink(vendor_dir, recursive = TRUE, force = TRUE)
    copy_tree(staging_vendor, vendor_dir)
    unlink(staging_vendor, recursive = TRUE, force = TRUE)
  }
}

rewrite_local_deps <- function(vendor_dir, local_crate_names) {
  for (name in local_crate_names) {
    manifest <- file.path(vendor_dir, name, "Cargo.toml")
    if (!file.exists(manifest)) next

    lines <- readLines(manifest, warn = FALSE)
    inserts <- list()

    for (sibling in local_crate_names) {
      if (sibling == name) next
      escaped <- regex_escape(sibling)

      for (j in seq_along(lines)) {
        line <- trimws(lines[[j]])

        # Table section: [dependencies.sibling] or [build-dependencies.sibling]
        section_pat <- sprintf("^\\[.*dependencies\\.%s\\]$", escaped)
        if (grepl(section_pat, line)) {
          has_path <- FALSE
          for (k in (j + 1L):min(j + 20L, length(lines))) {
            nxt <- trimws(lines[[k]])
            if (startsWith(nxt, "[")) break
            if (grepl("^path\\s*=", nxt)) { has_path <- TRUE; break }
          }
          if (!has_path) {
            inserts[[length(inserts) + 1L]] <- list(after = j, line = sprintf('path = "../%s"', sibling))
          }
          break
        }

        # Inline simple: sibling = "version"
        m <- regexec(sprintf("^(%s)\\s*=\\s*\"([^\"]+)\"$", escaped), line)
        if (length(regmatches(line, m)[[1]]) == 3) {
          ver <- regmatches(line, m)[[1]][[3]]
          lines[[j]] <- sprintf('%s = { version = "%s", path = "../%s" }', sibling, ver, sibling)
          break
        }

        # Inline table without path: sibling = { version = "...", ... }
        if (grepl(sprintf("^%s\\s*=\\s*\\{", escaped), line) &&
            grepl("version\\s*=", line) &&
            !grepl("path\\s*=", line)) {
          lines[[j]] <- sub("\\}\\s*$", sprintf(', path = "../%s" }', sibling), lines[[j]])
          break
        }
      }
    }

    if (length(inserts) > 0) {
      positions <- vapply(inserts, `[[`, integer(1), "after")
      for (ins in inserts[order(positions, decreasing = TRUE)]) {
        lines <- append(lines, ins$line, after = ins$after)
      }
    }

    writeLines(lines, manifest)
  }
}

build_vendor_tree <- function(package_root, source_root = NULL) {
  manifest_path <- file.path(package_root, "src", "rust", "Cargo.toml")
  if (!file.exists(manifest_path)) {
    stop(sprintf("Cargo manifest not found at %s", manifest_path), call. = FALSE)
  }

  source_root <- source_root %||% auto_source_root(package_root)
  if (!is.null(source_root)) {
    source_root <- normalize_existing(source_root)
  }

  staging_root <- tempfile("vendor-build-")
  dir.create(staging_root, recursive = TRUE)
  on.exit(unlink(staging_root, recursive = TRUE, force = TRUE), add = TRUE)

  local_roots <- discover_local_roots(manifest_path, package_root, source_root)
  packaged <- package_local_crates(local_roots, package_root, source_root, file.path(staging_root, "packaged"))

  staging_vendor <- file.path(staging_root, "vendor")
  dir.create(staging_vendor, recursive = TRUE)
  run_command(
    "cargo",
    c("vendor", "--manifest-path", manifest_path, staging_vendor),
    wd = package_root
  )

  if (nrow(packaged) > 0) {
    for (i in seq_len(nrow(packaged))) {
      extract_crate_archive(
        packaged$crate_file[[i]],
        staging_vendor,
        packaged$name[[i]],
        packaged$version[[i]]
      )
    }
    rewrite_local_deps(staging_vendor, packaged$name)
  }

  crate_dirs <- list.dirs(staging_vendor, recursive = FALSE, full.names = TRUE)
  for (crate_dir in crate_dirs) {
    strip_vendored_crate(crate_dir)
    checksum_file <- file.path(crate_dir, ".cargo-checksum.json")
    if (!file.exists(checksum_file)) {
      writeLines('{"files":{}}', checksum_file)
    }
  }

  if (!is.null(source_root)) {
    writeLines(source_root, file.path(staging_vendor, ".vendor-source"))
  }

  vendor_dir <- file.path(package_root, "vendor")
  swap_vendor_directory(staging_vendor, vendor_dir)

  invisible(list(
    vendor_dir = normalize_maybe(vendor_dir),
    source_root = source_root,
    local_crates = local_roots
  ))
}

vendor_sync <- function(package_root, source_root = NULL) {
  result <- build_vendor_tree(package_root, source_root)
  message(sprintf("Updated vendor tree at %s", result$vendor_dir))
  if (!is.null(result$source_root)) {
    message(sprintf("Recorded vendor source: %s", result$source_root))
  }
  invisible(result$vendor_dir)
}

vendor_pack <- function(package_root, source_root = NULL) {
  vendor_dir <- vendor_sync(package_root, source_root)
  strip_lockfile_checksums(file.path(package_root, "src", "rust", "Cargo.lock"))
  tarball <- compress_vendor_tarball(package_root, vendor_dir)
  message(sprintf("Created %s", tarball))
  invisible(tarball)
}

parse_args <- function(args) {
  command <- "pack"
  if (length(args) > 0 && !startsWith(args[[1]], "--")) {
    command <- args[[1]]
    args <- args[-1]
  }

  options <- list(
    command = command,
    path = dirname(script_dir),
    source_root = NULL
  )

  i <- 1L
  while (i <= length(args)) {
    arg <- args[[i]]

    if (arg %in% c("--path", "--source-root")) {
      if (i == length(args)) {
        stop(sprintf("Missing value for %s", arg), call. = FALSE)
      }
      value <- args[[i + 1L]]
      if (arg == "--path") {
        options$path <- value
      } else {
        options$source_root <- value
      }
      i <- i + 2L
      next
    }

    if (startsWith(arg, "--path=")) {
      options$path <- sub("^--path=", "", arg)
      i <- i + 1L
      next
    }

    if (startsWith(arg, "--source-root=")) {
      options$source_root <- sub("^--source-root=", "", arg)
      i <- i + 1L
      next
    }

    stop(sprintf("Unknown argument: %s", arg), call. = FALSE)
  }

  options
}

main <- function() {
  options <- parse_args(commandArgs(trailingOnly = TRUE))
  package_root <- normalize_existing(options$path)
  source_root <- options$source_root
  if (!is.null(source_root)) {
    source_root <- normalize_existing(source_root)
  }

  if (!options$command %in% c("pack", "sync")) {
    stop("Usage: vendor-crates.R [pack|sync] [--path PATH] [--source-root PATH]", call. = FALSE)
  }

  if (identical(options$command, "sync")) {
    vendor_sync(package_root, source_root)
  } else {
    vendor_pack(package_root, source_root)
  }
}

main()
