#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
extern crate git2;

use git2::{Repository, Error};
use git2::PushOptions;
use git2::{RemoteCallbacks, Cred};
use std::path::Path;

#[derive(Clone,PartialEq,Debug)]
pub struct GitAccess {
    file_url: String,
    pub repo_url: String,
    pub local_url: String,
    username: String,
    password: String,
}

impl GitAccess {
    pub fn new(repo: String, local_path: String, file_path: String, usern: String, passwd: String) -> GitAccess {
        GitAccess{repo_url: repo, local_url: local_path, file_url: file_path, username: usern, password: passwd}
    }

    pub fn clone_repo(&self) -> Result<(), git2::Error> {
        match Repository::clone(&self.repo_url, self.local_url.clone()) {
            Ok(repo) => repo,
            Err(e) => return Err(e)
        };
        Ok(())
    }

    pub fn commit_path(&self, commit_message: &str) -> Result<(), Error>  {
        println!("repo open {}", &self.local_url);
        let repo = match Repository::open(Path::new(&self.local_url)) {
            Ok(repo) => repo,
            Err(e) =>return Err(e)
        };
        let sig = try!(repo.signature());
        let tree_id = {
            let mut index = try!(repo.index());
            println!("adding path {}", &self.file_url);
            try!(index.add_path(Path::new(&self.file_url)));
            try!(index.write_tree_to(&repo))
        };

        let tree = try!(repo.find_tree(tree_id));
        // lookup current HEAD commit
        let head_ref = match repo.head() {
            Ok(head_ref) =>  head_ref,
            Err(e) => return Err(e)
        };
        let head_oid = head_ref.target().unwrap();
        let commit = try!(repo.find_commit(head_oid));
        // make that parent of new commit
        try!(repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &[&commit]));
        Ok(())
    }

    pub fn push(&self) -> Result<(), git2::Error> {
        let repo = match Repository::open(Path::new(&self.local_url)) {
            Ok(repo) => repo,
            Err(e) => return Err(e)
        };

        let mut cb = RemoteCallbacks::new();
        cb.credentials(|_, _, _| {  // |repoName, options, cred_type|
            // get credentials from user
            Cred::userpass_plaintext(&self.username, &self.password)
        });
        let remote = "origin";
        let mut remote = try!(repo.find_remote(remote));
        let mut opt_push = PushOptions::new();
        opt_push.remote_callbacks(cb);
        let x: Option<&mut PushOptions> = Some(&mut opt_push);
        match remote.push(&["refs/heads/master"], x) {
            Ok(p) => p,
            Err(e) => return Err(e)
        };

        Ok(())
    }
}

/*
pub fn testing_func() {
    let url = "D:\\DS\\Project\\dummyRepo";
    let repo_url = "https://gitub.com/roshanib/dummyRepo.git";
    let file_path = "permissions.txt";
    let username = "rbhandari1";
    let password = "password"; //TODO
    let ga = GitAccess::new(repo_url, username, password);
    match ga.clone(`repo_url, url) {
        Ok(()) => {},
        Err(e) => println!("error: {}", e),
    };
    match ga.commit_path(url, "demo commit rbhandari1", file_path) {
        Ok(()) => {},
        Err(e) => println!("error: {}", e),
    };
    match ga.push() {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
*/
