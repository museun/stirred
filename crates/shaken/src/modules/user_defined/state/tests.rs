use super::*;

fn make_command(name: &str) -> Command {
    Command {
        name: name.to_string(),
        body: String::from("testing"),
        author: String::from("testuser"),
        uses: 0,
    }
}

#[test]
fn insert() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert!(!state.insert(make_command("!foo")));
    assert!(state.insert(make_command("!bar")));
}

#[test]
fn remove() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert!(state.remove("!foo"));
    assert!(!state.remove("!foo"));
}

#[test]
fn update() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert_eq!(state.get_by_name("!foo").unwrap().body, "testing");
    assert!(state.update("!foo", |cmd| cmd.body = String::from("hello")));
    assert_eq!(state.get_by_name("!foo").unwrap().body, "hello");
}

#[test]
fn alias() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert!(state.alias("!foo", "!bar"));
    assert_eq!(state.get_by_name("!foo").unwrap().body, "testing");
    assert_eq!(state.get_by_name("!bar").unwrap().body, "testing");
}

#[test]
fn get_by_name() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert!(state.alias("!foo", "!bar"));
    assert!(state.insert(make_command("!baz")));

    state.get_by_name("!foo").unwrap();
    state.get_by_name("!baz").unwrap();

    state.get_by_name("!bar").unwrap();
}

#[test]
fn get_all() {
    let mut state = UserDefinedState::default();
    assert!(state.insert(make_command("!foo")));
    assert!(state.alias("!foo", "!bar"));
    assert!(state.insert(make_command("!baz")));

    let mut left = state.get_all().collect::<Vec<_>>();
    left.sort();

    assert_eq!(left, vec![&make_command("!baz"), &make_command("!foo")]);
}
