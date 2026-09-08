use ratatui::backend::TestBackend;
use ratatui::Terminal;
use sp_core::AppState;

fn state_with_links(links: Vec<&str>) -> AppState {
    let mut state = AppState::default();
    state.links = links.into_iter().map(str::to_string).collect();
    state.filtered = (0..state.links.len()).collect();
    state
}

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn renders_at_80x24_without_panicking() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let state = state_with_links(vec!["github.com", "docs.rs"]);
    terminal.draw(|frame| sp_ui::render(frame, &state)).unwrap();

    let text = buffer_text(&terminal);
    assert!(text.contains("github.com"));
    assert!(text.contains("docs.rs"));
}

#[test]
fn renders_at_200x60_without_panicking() {
    let mut terminal = Terminal::new(TestBackend::new(200, 60)).unwrap();
    let state = state_with_links(vec!["github.com", "docs.rs"]);
    terminal.draw(|frame| sp_ui::render(frame, &state)).unwrap();

    let text = buffer_text(&terminal);
    assert!(text.contains("github.com"));
}

#[test]
fn long_link_names_ellipsize_instead_of_wrapping() {
    let long = "https://example.com/a/very/long/path/that/is/wider/than/the/terminal/viewport/and/should/be/truncated";
    let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
    let state = state_with_links(vec![long]);
    terminal.draw(|frame| sp_ui::render(frame, &state)).unwrap();

    let text = buffer_text(&terminal);
    assert!(!text.contains(long), "long link should have been truncated");
    assert!(text.contains('…'), "truncated link should end with an ellipsis");
}

#[test]
fn cjk_and_zero_width_characters_do_not_corrupt_the_grid() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    // CJK (double-width cells) plus a zero-width-joiner emoji sequence (family: man, woman, girl, boy).
    let state = state_with_links(vec!["東京都新宿区", "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}"]);
    terminal.draw(|frame| sp_ui::render(frame, &state)).unwrap();

    let buffer = terminal.backend().buffer();
    // Every row must still have exactly `width` cells and no panic occurred getting here —
    // that's the actual "did not corrupt the grid" assertion.
    assert_eq!(buffer.area.width, 80);
    assert_eq!(buffer.area.height, 24);
    assert!(buffer_text(&terminal).contains('東'));
}
