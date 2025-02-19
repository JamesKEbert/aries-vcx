pub fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
