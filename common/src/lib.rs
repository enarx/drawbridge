pub const APP_NAME: &str = "drawbridge";

pub const COOKIE_NAME: &str = "SESSION";

pub mod endpoint {
    // Auth routes
    macro_rules! auth_root {
        () => {
            "/auth"
        };
    }

    pub static STATUS: &str = concat!(auth_root!(), "/status");
    pub static GITHUB: &str = concat!(auth_root!(), "/github");
    pub static GITHUB_AUTHORIZED: &str = concat!(auth_root!(), "/github/authorized");

    // Protected routes
    pub const PROTECTED: &str = "/protected";
}
