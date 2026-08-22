use crate::config::AuthMethodConfig;
use crate::models::{AuthMethod, ServerConnection};

/// Default SSH key used for Public Key auth when no path is entered.
const DEFAULT_KEY_PATH: &str = "~/.ssh/id_rsa";
/// Index of the SSH key path field within `ServerForm::fields`.
const KEY_PATH_FIELD: usize = 5;
/// Index of the connect-timeout field within `ServerForm::fields`.
const TIMEOUT_FIELD: usize = 6;

/// Represents a text input field in a form.
///
/// `cursor_position` is a **character** index, not a byte index. All mutation
/// goes through `byte_offset()`, which maps that char index onto a real UTF-8
/// boundary. Tracking bytes directly (the previous behaviour) panicked the
/// moment a user typed any multi-byte character — `String::insert` asserts on
/// non-char-boundary indices.
#[derive(Debug, Clone)]
pub struct InputField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
    /// Cursor position measured in characters (0..=char_count).
    pub cursor_position: usize,
    pub is_password: bool,
}

impl InputField {
    pub fn new(label: &str, placeholder: &str) -> Self {
        Self {
            label: label.to_string(),
            value: String::new(),
            placeholder: placeholder.to_string(),
            is_focused: false,
            cursor_position: 0,
            is_password: false,
        }
    }

    /// Number of characters (not bytes) in the value.
    pub fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    /// Translate the char-index cursor into a byte offset into `value`.
    /// Always lands on a char boundary, so `String::insert`/`remove` are safe.
    fn byte_offset(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.value.len())
    }

    /// Clamp the cursor back into range — used after the value is replaced
    /// wholesale (e.g. when populating an edit form).
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_position = self.char_count();
    }

    pub fn insert_char(&mut self, c: char) {
        let at = self.byte_offset(self.cursor_position);
        self.value.insert(at, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let at = self.byte_offset(self.cursor_position - 1);
            self.value.remove(at);
            self.cursor_position -= 1;
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.char_count() {
            let at = self.byte_offset(self.cursor_position);
            self.value.remove(at);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.char_count() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.char_count();
    }

    pub fn display_value(&self) -> String {
        if self.is_password && !self.value.is_empty() {
            "*".repeat(self.char_count())
        } else {
            self.value.clone()
        }
    }

    /// Terminal columns occupied by the text before the cursor. Wide glyphs
    /// (CJK, many emoji) take two cells, so this is what the renderer needs to
    /// place the caret — a plain char count would drift.
    pub fn cursor_display_column(&self) -> usize {
        use unicode_width::UnicodeWidthChar;
        if self.is_password {
            return self.cursor_position;
        }
        self.value
            .chars()
            .take(self.cursor_position)
            .map(|c| c.width().unwrap_or(0))
            .sum()
    }
}

/// Authentication method selection for forms
#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethodSelection {
    Agent,
    Password,
    PublicKey,
    Interactive,
}

impl AuthMethodSelection {
    pub fn variants() -> Vec<AuthMethodSelection> {
        vec![
            AuthMethodSelection::Agent,
            AuthMethodSelection::Password,
            AuthMethodSelection::PublicKey,
            AuthMethodSelection::Interactive,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AuthMethodSelection::Agent => "SSH Agent",
            AuthMethodSelection::Password => "Password",
            AuthMethodSelection::PublicKey => "Public Key",
            AuthMethodSelection::Interactive => "Interactive",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AuthMethodSelection::Agent => "Use SSH agent for authentication",
            AuthMethodSelection::Password => "Use password authentication (not recommended)",
            AuthMethodSelection::PublicKey => "Use public key authentication",
            AuthMethodSelection::Interactive => "Interactive keyboard authentication",
        }
    }
}

impl From<AuthMethodSelection> for AuthMethodConfig {
    fn from(selection: AuthMethodSelection) -> Self {
        match selection {
            AuthMethodSelection::Agent => AuthMethodConfig::Agent,
            AuthMethodSelection::Password => AuthMethodConfig::Password,
            AuthMethodSelection::PublicKey => AuthMethodConfig::PublicKey {
                key_path: "~/.ssh/id_rsa".to_string(),
            },
            AuthMethodSelection::Interactive => AuthMethodConfig::Interactive,
        }
    }
}

impl From<&AuthMethod> for AuthMethodSelection {
    fn from(auth: &AuthMethod) -> Self {
        match auth {
            AuthMethod::Agent => AuthMethodSelection::Agent,
            AuthMethod::Password => AuthMethodSelection::Password,
            AuthMethod::PublicKey { .. } => AuthMethodSelection::PublicKey,
            AuthMethod::Interactive => AuthMethodSelection::Interactive,
        }
    }
}

/// Server form state for adding/editing servers
#[derive(Debug, Clone)]
pub struct ServerForm {
    pub fields: Vec<InputField>,
    pub auth_method: AuthMethodSelection,
    pub auth_method_focused: bool,
    pub current_field: usize,
    pub is_editing: bool,
    pub original_id: Option<String>,
    pub tags_input: InputField,
}

impl ServerForm {
    /// Create a new form for adding a server
    pub fn new_add_form() -> Self {
        let fields = vec![
            InputField::new("Name", "My Server"),
            InputField::new("Host", "example.com"),
            InputField::new("Port", "22"),
            InputField::new("Username", "user"),
            InputField::new("Description", "Optional description"),
            // Only used when the auth method is "Public Key"; left blank falls
            // back to the default key below.
            InputField::new("SSH Key Path (Public Key auth)", DEFAULT_KEY_PATH),
            // Blank means "use the built-in default".
            InputField::new("Connect Timeout (seconds)", "10"),
        ];

        let mut tags_input = InputField::new("Tags", "web,production");
        tags_input.value = String::new();

        let mut form = Self {
            fields,
            auth_method: AuthMethodSelection::Agent,
            auth_method_focused: false,
            current_field: 0,
            is_editing: false,
            original_id: None,
            tags_input,
        };
        form.update_focus();
        form
    }

    /// Create a form for editing an existing server
    pub fn new_edit_form(connection: &ServerConnection) -> Self {
        let mut form = Self::new_add_form();
        form.is_editing = true;
        form.original_id = Some(connection.id.clone());

        // Populate fields. `set_value` also parks the cursor at the end using a
        // char count, so multi-byte names/descriptions stay editable.
        form.fields[0].set_value(connection.name.clone());
        form.fields[1].set_value(connection.host.clone());
        form.fields[2].set_value(connection.port.to_string());
        form.fields[3].set_value(connection.username.clone());

        if let Some(desc) = &connection.description {
            form.fields[4].set_value(desc.clone());
        }

        // Pre-fill the key path when editing a Public Key connection.
        if let AuthMethod::PublicKey { key_path } = &connection.auth_method {
            form.fields[KEY_PATH_FIELD].set_value(key_path.clone());
        }

        if let Some(timeout) = connection.timeout {
            form.fields[TIMEOUT_FIELD].set_value(timeout.to_string());
        }

        // Set auth method
        form.auth_method = AuthMethodSelection::from(&connection.auth_method);

        // Set tags
        form.tags_input.set_value(connection.tags.join(","));

        form.update_focus();
        form
    }

    /// Get the currently focused input field
    pub fn current_field_mut(&mut self) -> Option<&mut InputField> {
        if self.auth_method_focused {
            None // Auth method dropdown is focused
        } else if self.current_field == self.fields.len() {
            Some(&mut self.tags_input) // Tags field is focused
        } else {
            self.fields.get_mut(self.current_field)
        }
    }

    /// Focus order, as a flat list of rows:
    ///   `fields[0..n]`, then tags, then the auth-method selector.
    ///
    /// Modelling it linearly fixes a wrap-around bug: `next_field` used to jump
    /// from tags straight back to field 0, so Tab could never reach the auth
    /// selector — it was only reachable by shift-tabbing backwards off the
    /// first field.
    fn row_count(&self) -> usize {
        self.fields.len() + 2
    }

    fn auth_row(&self) -> usize {
        self.row_count() - 1
    }

    fn tags_row(&self) -> usize {
        self.fields.len()
    }

    /// Current position in the flat focus order.
    fn focus_row(&self) -> usize {
        if self.auth_method_focused {
            self.auth_row()
        } else {
            self.current_field.min(self.tags_row())
        }
    }

    fn set_focus_row(&mut self, row: usize) {
        if row == self.auth_row() {
            self.auth_method_focused = true;
        } else {
            self.auth_method_focused = false;
            self.current_field = row;
        }
        self.update_focus();
    }

    /// Move focus to the next field, wrapping at the end.
    pub fn next_field(&mut self) {
        let next = (self.focus_row() + 1) % self.row_count();
        self.set_focus_row(next);
    }

    /// Move focus to the previous field, wrapping at the start.
    pub fn previous_field(&mut self) {
        let count = self.row_count();
        let prev = (self.focus_row() + count - 1) % count;
        self.set_focus_row(prev);
    }

    /// Update field focus states
    fn update_focus(&mut self) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.is_focused = i == self.current_field && !self.auth_method_focused;
        }
        self.tags_input.is_focused =
            self.current_field == self.fields.len() && !self.auth_method_focused;
    }

    /// Select next auth method
    pub fn next_auth_method(&mut self) {
        let variants = AuthMethodSelection::variants();
        let current_index = variants
            .iter()
            .position(|x| *x == self.auth_method)
            .unwrap_or(0);
        let next_index = (current_index + 1) % variants.len();
        self.auth_method = variants[next_index].clone();
    }

    /// Select previous auth method
    pub fn previous_auth_method(&mut self) {
        let variants = AuthMethodSelection::variants();
        let current_index = variants
            .iter()
            .position(|x| *x == self.auth_method)
            .unwrap_or(0);
        let prev_index = if current_index == 0 {
            variants.len() - 1
        } else {
            current_index - 1
        };
        self.auth_method = variants[prev_index].clone();
    }

    /// Validate the form and return errors if any
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.fields[0].value.trim().is_empty() {
            errors.push("Name is required".to_string());
        }

        if self.fields[1].value.trim().is_empty() {
            errors.push("Host is required".to_string());
        }

        match self.fields[2].value.trim() {
            "" => errors.push("Port is required".to_string()),
            port => match port.parse::<u16>() {
                // Port 0 parses fine as a u16 but is not a connectable port.
                Ok(0) => errors.push("Port must be between 1 and 65535".to_string()),
                Ok(_) => {}
                Err(_) => errors.push("Port must be a number between 1 and 65535".to_string()),
            },
        }

        if self.fields[3].value.trim().is_empty() {
            errors.push("Username is required".to_string());
        }

        match self.fields[TIMEOUT_FIELD].value.trim() {
            "" => {}
            t => match t.parse::<u64>() {
                Ok(0) => errors.push("Timeout must be at least 1 second".to_string()),
                Ok(n) if n > 300 => errors.push("Timeout must be 300 seconds or less".to_string()),
                Ok(_) => {}
                Err(_) => errors.push("Timeout must be a whole number of seconds".to_string()),
            },
        }

        errors
    }

    /// Convert form data to ServerConnection
    pub fn to_server_connection(&self) -> Result<ServerConnection, String> {
        let errors = self.validate();
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }

        let port = self.fields[2]
            .value
            .parse::<u16>()
            .map_err(|_| "Invalid port number".to_string())?;

        let mut connection = ServerConnection::new(
            self.fields[0].value.trim().to_string(),
            self.fields[1].value.trim().to_string(),
            port,
            self.fields[3].value.trim().to_string(),
        );

        // Set description if provided
        if !self.fields[4].value.trim().is_empty() {
            connection.description = Some(self.fields[4].value.trim().to_string());
        }

        // Set auth method
        let auth_config: AuthMethodConfig = self.auth_method.clone().into();

        // For public key, use the path entered in the form, falling back to the
        // default key when the field is left blank.
        let auth_method = match &auth_config {
            AuthMethodConfig::PublicKey { .. } => {
                let entered = self.fields[KEY_PATH_FIELD].value.trim();
                let key_path = if entered.is_empty() {
                    DEFAULT_KEY_PATH.to_string()
                } else {
                    entered.to_string()
                };
                AuthMethod::PublicKey { key_path }
            }
            _ => auth_config.into(),
        };
        connection.auth_method = auth_method;

        // Blank timeout means "use the default", which is stored as None.
        connection.timeout = self.fields[TIMEOUT_FIELD].value.trim().parse::<u64>().ok();

        // Set tags
        if !self.tags_input.value.trim().is_empty() {
            connection.tags = self
                .tags_input
                .value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // If editing, preserve the original ID.
        if let Some(ref original_id) = self.original_id {
            connection.id = original_id.clone();
        }

        Ok(connection)
    }

    /// Check if form has any input
    pub fn has_input(&self) -> bool {
        self.fields.iter().any(|f| !f.value.is_empty()) || !self.tags_input.value.is_empty()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> InputField {
        let mut f = InputField::new("t", "");
        for c in s.chars() {
            f.insert_char(c);
        }
        f
    }

    #[test]
    fn multibyte_input_does_not_panic() {
        // Regression: cursor_position used to be a byte index advanced by one
        // per char, so the second keystroke after any multi-byte character
        // landed mid-codepoint and `String::insert` aborted the process.
        let f = typed("héllo wörld");
        assert_eq!(f.value, "héllo wörld");
        assert_eq!(f.cursor_position, 11);
    }

    #[test]
    fn emoji_and_cjk_survive_editing() {
        let mut f = typed("日本語🚀");
        assert_eq!(f.cursor_position, 4);
        f.delete_char();
        assert_eq!(f.value, "日本語");
        f.move_cursor_to_start();
        f.insert_char('x');
        assert_eq!(f.value, "x日本語");
        assert_eq!(f.cursor_position, 1);
    }

    #[test]
    fn cursor_moves_by_characters_not_bytes() {
        let mut f = typed("aé");
        f.move_cursor_to_start();
        f.move_cursor_right();
        assert_eq!(f.cursor_position, 1);
        f.delete_char_forward(); // removes 'é' whole
        assert_eq!(f.value, "a");
        // Cursor cannot run past the end.
        f.move_cursor_right();
        f.move_cursor_right();
        assert_eq!(f.cursor_position, 1);
    }

    #[test]
    fn wide_glyphs_take_two_display_columns() {
        let f = typed("日本");
        assert_eq!(f.cursor_position, 2);
        assert_eq!(f.cursor_display_column(), 4);
    }

    #[test]
    fn delete_on_empty_field_is_a_noop() {
        let mut f = InputField::new("t", "");
        f.delete_char();
        f.delete_char_forward();
        f.move_cursor_left();
        assert_eq!(f.value, "");
        assert_eq!(f.cursor_position, 0);
    }

    #[test]
    fn tab_order_reaches_every_row_including_auth() {
        // Regression: Tab used to wrap from the tags field back to field 0,
        // so the auth-method selector was unreachable going forward.
        let mut form = ServerForm::new_add_form();
        let rows = form.fields.len() + 2;

        let mut saw_auth = false;
        let mut saw_tags = false;
        for _ in 0..rows {
            form.next_field();
            if form.auth_method_focused {
                saw_auth = true;
            } else if form.current_field == form.fields.len() {
                saw_tags = true;
            }
        }
        assert!(saw_auth, "Tab never reached the auth selector");
        assert!(saw_tags, "Tab never reached the tags field");
    }

    #[test]
    fn focus_order_is_a_cycle_in_both_directions() {
        let mut form = ServerForm::new_add_form();
        let rows = form.fields.len() + 2;
        for _ in 0..rows {
            form.next_field();
        }
        assert!(!form.auth_method_focused);
        assert_eq!(form.current_field, 0);

        for _ in 0..rows {
            form.previous_field();
        }
        assert!(!form.auth_method_focused);
        assert_eq!(form.current_field, 0);
    }

    #[test]
    fn validation_rejects_bad_ports_and_timeouts() {
        let mut form = ServerForm::new_add_form();
        form.fields[0].set_value("n".into());
        form.fields[1].set_value("h".into());
        form.fields[3].set_value("u".into());

        form.fields[2].set_value("0".into());
        assert!(form.validate().iter().any(|e| e.contains("Port")));

        form.fields[2].set_value("70000".into());
        assert!(form.validate().iter().any(|e| e.contains("Port")));

        form.fields[2].set_value("22".into());
        assert!(form.validate().is_empty());

        form.fields[6].set_value("0".into());
        assert!(form.validate().iter().any(|e| e.contains("Timeout")));

        form.fields[6].set_value("abc".into());
        assert!(form.validate().iter().any(|e| e.contains("Timeout")));

        form.fields[6].set_value("30".into());
        assert!(form.validate().is_empty());
    }

    #[test]
    fn timeout_round_trips_through_the_form() {
        let mut form = ServerForm::new_add_form();
        form.fields[0].set_value("n".into());
        form.fields[1].set_value("h".into());
        form.fields[2].set_value("22".into());
        form.fields[3].set_value("u".into());
        form.fields[6].set_value("45".into());

        let conn = form.to_server_connection().unwrap();
        assert_eq!(conn.timeout, Some(45));

        // A blank timeout means "use the default", stored as None.
        form.fields[6].set_value(String::new());
        assert_eq!(form.to_server_connection().unwrap().timeout, None);
    }

    #[test]
    fn editing_preserves_the_original_id() {
        let mut original = ServerConnection::new("n".into(), "h".into(), 22, "u".into());
        original.timeout = Some(20);
        let form = ServerForm::new_edit_form(&original);
        let edited = form.to_server_connection().unwrap();
        assert_eq!(edited.id, original.id);
        assert_eq!(edited.timeout, Some(20));
    }

    #[test]
    fn password_masking_counts_characters() {
        let mut f = typed("pässwörd");
        f.is_password = true;
        assert_eq!(f.display_value(), "*".repeat(8));
    }
}
