pub use nix_search_core::parse::{parse_dump, parse_key};

#[cfg(test)]
mod tests {
    use super::{parse_dump, parse_key};

    #[test]
    fn parse_key_works() {
        let (plat, attr) =
            parse_key("legacyPackages.x86_64-linux.python312Packages.requests").unwrap();
        assert_eq!(plat, "x86_64-linux");
        assert_eq!(attr, "python312Packages.requests");
    }

    #[test]
    fn parse_dump_groups_by_attr() {
        let json = r#"{
            "legacyPackages.x86_64-linux.foo": {"pname":"foo","version":"1","description":"d"},
            "legacyPackages.aarch64-darwin.foo": {"pname":"foo","version":"1","description":"d"}
        }"#;

        let pkgs = parse_dump(json).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].attr_path, "foo");
        assert_eq!(pkgs[0].platforms.len(), 2);
    }
}
