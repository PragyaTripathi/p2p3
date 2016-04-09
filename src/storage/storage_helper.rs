#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
extern crate git2;

use git2::{Repository, Error};
use git2::PushOptions;
use git2::{RemoteCallbacks, Cred};
use std::path::Path;


pub fn clone(src_repo: &str, dst_dir: &str) -> Result<(), git2::Error> {
    match Repository::clone(src_repo, dst_dir) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };
    Ok(())
}

pub fn commit_path(url: &str, commit_message: &str, file_path: &str) -> Result<(), Error>  {
    let repo = match Repository::open(url) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open {}", e),
    };
    let sig = try!(repo.signature());
    let tree_id = {
        let mut index = try!(repo.index());
        try!(index.add_path(Path::new(file_path)));
        try!(index.write_tree_to(&repo))
    };

    let tree = try!(repo.find_tree(tree_id));
    // lookup current HEAD commit
    let head_ref = match repo.head() {
        Ok(head_ref) =>  head_ref,
        Err(e) => panic!("Error getting head"),
    };
    let head_oid = head_ref.target().unwrap();
    let commit = try!(repo.find_commit(head_oid));
    // make that parent of new commit
    try!(repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &[&commit]));
    Ok(())
}

pub fn push(url: &str, username: &str, password: &str) -> Result<(), git2::Error> {
    let repo = match Repository::open(url) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open {}", e),
    };

    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_, _, _| {  // |repoName, options, cred_type|
        // get credentials from user
        Cred::userpass_plaintext(username, password)
    });
    let remote = "origin";
    let mut remote = try!(repo.find_remote(remote));
    let mut opt_push = PushOptions::new();
    opt_push.remote_callbacks(cb);
    let x: Option<&mut PushOptions> = Some(&mut opt_push);
    match remote.push(&["refs/heads/master"], x) {
        Ok(p) => p,
        Err(e) => panic!("Failed to push: {}", e),
    };

    Ok(())
}

fn testing_func() {
    let url = "D:\\DS\\Project\\dummyRepo";
    let repo_url = "https://github.com/roshanib/dummyRepo.git";
    let file_path = "permissions.txt";
    let username = "roshanib";
    let password = "password"; //TODO
    match clone(repo_url, url) {
        Ok(()) => {},
        Err(e) => println!("error: {}", e),
    };
    match commit_path(url, "demo commit", file_path) {
        Ok(()) => {},
        Err(e) => println!("error: {}", e),
    };
    match push(url, username, password) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }

}
