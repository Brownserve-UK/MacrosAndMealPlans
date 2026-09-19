use super::*;

#[test]
fn the_default_port_avoids_the_crowded_ones() {
    let address: SocketAddr = DEFAULT_BIND_ADDRESS.parse().unwrap();
    let port = address.port();
    assert_eq!(port, 7979);
    for crowded in [80, 3000, 5000, 8000, 8080, 8081, 8888, 9000] {
        assert_ne!(port, crowded, "{crowded} is too commonly used");
    }
}

#[test]
fn flags_accept_the_usual_spellings() {
    unsafe { env::set_var("MMP_TEST_FLAG", "TRUE") };
    assert!(flag("MMP_TEST_FLAG", false).unwrap());
    unsafe { env::set_var("MMP_TEST_FLAG", "off") };
    assert!(!flag("MMP_TEST_FLAG", true).unwrap());
    unsafe { env::remove_var("MMP_TEST_FLAG") };
    assert!(flag("MMP_TEST_FLAG", true).unwrap());
}

#[test]
fn a_nonsense_flag_is_rejected() {
    unsafe { env::set_var("MMP_TEST_BAD_FLAG", "perhaps") };
    assert!(flag("MMP_TEST_BAD_FLAG", true).is_err());
    unsafe { env::remove_var("MMP_TEST_BAD_FLAG") };
}
