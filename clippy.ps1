#  cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::expect_used
#  cargo watch -x "clippy --bin event_hub_input -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used"
# cargo watch -x "clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used"
cargo watch -x "clippy -- -W clippy::pedantic -W clippy::unwrap_used"
# cargo watch -x "clippy -- -W clippy::pedantic"
# cargo watch -x "clippy -- -W clippy::pedantic -W clippy::nursery"
# cargo watch -x "clippy -- -W clippy::expect_used"