use std::sync::{Once, ONCE_INIT};
use std::mem;
use storage::storage_helper::GitAccess;

#[derive(Clone)]
pub struct P2P3Globals {
    site_id: u32,
    port: u16,
    url: String,
    git_access: GitAccess
}

impl P2P3Globals {
    pub fn init_globals(&mut self, site_id: u32, port: u16, url: String, git_access: GitAccess) {
        self.site_id = site_id;
        self.port = port;
        self.url = url;
        self.git_access = git_access;
    }

    pub fn get_site_id(&self) -> u32 {
        self.site_id
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_git_access(&self) -> GitAccess {
        self.git_access.clone()
    }
}

pub fn p2p3_globals() -> P2P3Globals {
    // Initialize it to a null value
    static mut SINGLETON: *const P2P3Globals = 0 as *const P2P3Globals;
    static ONCE: Once = ONCE_INIT;

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = P2P3Globals {
                site_id: 0,
                port: 8080,
                url: String::new(),
                git_access: GitAccess::new(String::new(),String::new(),String::new(),String::new(),String::new())
            };

            // Put it in the heap so it can outlive this call
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        return (*SINGLETON).clone();
    }
}
