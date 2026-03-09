#![allow(clippy::unwrap_used)]

use crate::git::diff::{ChangedFile, FileStatus};
use crate::tree::TreeDir;

fn make_file(path: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_owned(),
        status: FileStatus::Modified,
        staged: false,
        unstaged: true,
    }
}

#[test]
fn collapse_single_child_chain() {
    let mut root = TreeDir::root();
    let file = make_file("a/b/c/file.txt");
    root.insert_file(0, &file);
    root.sort_files_recursive();
    root.collapse_single_child_dirs();

    assert_eq!(root.dirs.len(), 1);
    let collapsed = root.dirs.values().next().unwrap();
    assert_eq!(collapsed.name, "a/b/c");
    assert_eq!(collapsed.path, "a/b/c");
    assert_eq!(collapsed.files.len(), 1);
    assert_eq!(collapsed.files[0].name, "file.txt");
    assert!(collapsed.dirs.is_empty());
}

#[test]
fn no_collapse_when_multiple_children() {
    let mut root = TreeDir::root();
    root.insert_file(0, &make_file("src/main.rs"));
    root.insert_file(1, &make_file("src/lib.rs"));
    root.insert_file(2, &make_file("src/utils/helper.rs"));
    root.sort_files_recursive();
    root.collapse_single_child_dirs();

    let src = &root.dirs["src"];
    assert_eq!(src.name, "src");
    assert_eq!(src.files.len(), 2);
    assert_eq!(src.dirs.len(), 1);
}

#[test]
fn partial_collapse() {
    let mut root = TreeDir::root();
    root.insert_file(0, &make_file("a/b/c/file1.txt"));
    root.insert_file(1, &make_file("a/b/d/file2.txt"));
    root.sort_files_recursive();
    root.collapse_single_child_dirs();

    let ab = root.dirs.values().next().unwrap();
    assert_eq!(ab.name, "a/b");
    assert_eq!(ab.path, "a/b");
    assert_eq!(ab.dirs.len(), 2);
}

#[test]
fn collapse_preserves_expanded_dir_paths() {
    let mut root = TreeDir::root();
    root.insert_file(0, &make_file("internal/api/handlers/management/handler.go"));
    root.sort_files_recursive();
    root.collapse_single_child_dirs();

    let collapsed = root.dirs.values().next().unwrap();
    assert_eq!(collapsed.name, "internal/api/handlers/management");
    assert_eq!(collapsed.path, "internal/api/handlers/management");
}

#[test]
fn collapse_does_not_merge_dir_with_files() {
    let mut root = TreeDir::root();
    root.insert_file(0, &make_file("a/readme.md"));
    root.insert_file(1, &make_file("a/b/deep.txt"));
    root.sort_files_recursive();
    root.collapse_single_child_dirs();

    let a = &root.dirs["a"];
    assert_eq!(a.name, "a");
    assert_eq!(a.dirs.len(), 1);
    assert_eq!(a.files.len(), 1);
}
