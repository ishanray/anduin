use crate::git::diff::{ChangedFile, FileStatus};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub(crate) struct TreeFile {
    pub index: usize,
    pub name: String,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TreeDir {
    pub name: String,
    pub path: String,
    pub dirs: BTreeMap<String, Self>,
    pub files: Vec<TreeFile>,
}

#[derive(Debug, Clone)]
pub(crate) enum SidebarRow {
    Root {
        name: String,
        expanded: bool,
    },
    Dir {
        name: String,
        path: String,
        depth: usize,
        expanded: bool,
    },
    File {
        name: String,
        index: usize,
        depth: usize,
        status: FileStatus,
    },
}

impl TreeDir {
    pub(crate) fn root() -> Self {
        Self::default()
    }

    pub(crate) fn insert_file(&mut self, index: usize, file: &ChangedFile) {
        let parts: Vec<&str> = file.path.split('/').collect();
        self.insert_parts(index, file, &parts, String::new());
    }

    pub(crate) fn sort_files_recursive(&mut self) {
        self.files.sort_by(|a, b| a.name.cmp(&b.name));
        for dir in self.dirs.values_mut() {
            dir.sort_files_recursive();
        }
    }

    fn insert_parts(
        &mut self,
        index: usize,
        file: &ChangedFile,
        parts: &[&str],
        current_path: String,
    ) {
        if parts.len() == 1 {
            self.files.push(TreeFile {
                index,
                name: parts[0].to_owned(),
                status: file.status,
            });
            return;
        }

        let dir_name = parts[0];
        let dir_path = if current_path.is_empty() {
            dir_name.to_owned()
        } else {
            format!("{}/{}", current_path, dir_name)
        };

        let dir = self
            .dirs
            .entry(dir_name.to_owned())
            .or_insert_with(|| Self {
                name: dir_name.to_owned(),
                path: dir_path.clone(),
                ..Self::default()
            });

        dir.insert_parts(index, file, &parts[1..], dir_path);
    }

    /// Collapse chains of single-child directories into one node.
    /// e.g. `a -> b -> c -> file.txt` becomes `a/b/c -> file.txt`
    pub(crate) fn collapse_single_child_dirs(&mut self) {
        // First, recursively collapse children
        for dir in self.dirs.values_mut() {
            dir.collapse_single_child_dirs();
        }

        // Then collapse at this level: if a child dir is the only child
        // (no sibling dirs, no files at this level) and itself has no files,
        // merge it upward
        let dir_keys: Vec<String> = self.dirs.keys().cloned().collect();
        for key in dir_keys {
            let should_merge = {
                let dir = &self.dirs[&key];
                dir.dirs.len() == 1 && dir.files.is_empty()
            };

            if should_merge
                && let Some(dir) = self.dirs.remove(&key)
                && let Some((_, child)) = dir.dirs.into_iter().next()
            {
                let merged = Self {
                    name: format!("{}/{}", dir.name, child.name),
                    path: child.path,
                    dirs: child.dirs,
                    files: child.files,
                };
                let merged_key = merged.name.clone();
                self.dirs.insert(merged_key, merged);
            }
        }
    }

    pub(crate) fn collect_dir_paths(&self, paths: &mut Vec<String>) {
        for dir in self.dirs.values() {
            paths.push(dir.path.clone());
            dir.collect_dir_paths(paths);
        }
    }

    pub(crate) fn collect_visible_rows(
        &self,
        expanded_dirs: &HashSet<String>,
        depth: usize,
        rows: &mut Vec<SidebarRow>,
    ) {
        for dir in self.dirs.values() {
            let expanded = expanded_dirs.contains(&dir.path);
            rows.push(SidebarRow::Dir {
                name: dir.name.clone(),
                path: dir.path.clone(),
                depth,
                expanded,
            });

            if expanded {
                dir.collect_visible_rows(expanded_dirs, depth + 1, rows);
            }
        }

        for file in &self.files {
            rows.push(SidebarRow::File {
                name: file.name.clone(),
                index: file.index,
                depth,
                status: file.status,
            });
        }
    }
}

pub(crate) fn expand_parent_dirs(expanded_dirs: &mut HashSet<String>, file_path: &str) {
    let mut current = String::new();
    let mut parts = file_path.split('/').peekable();

    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            break;
        }

        if current.is_empty() {
            current.push_str(part);
        } else {
            current.push('/');
            current.push_str(part);
        }

        expanded_dirs.insert(current.clone());
    }
}
