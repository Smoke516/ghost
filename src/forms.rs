use crate::config::AuthMethodConfig;
use crate::models::{AuthMethod, ServerConnection};

/// Represents a text input field in a form
#[derive(Debug, Clone)]
pub struct InputField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
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



    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.value.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    pub fn display_value(&self) -> String {
        if self.is_password && !self.value.is_empty() {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        }
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

        // Populate fields
        form.fields[0].value = connection.name.clone();
        form.fields[0].cursor_position = connection.name.len();

        form.fields[1].value = connection.host.clone();
        form.fields[1].cursor_position = connection.host.len();

        form.fields[2].value = connection.port.to_string();
        form.fields[2].cursor_position = connection.port.to_string().len();

        form.fields[3].value = connection.username.clone();
        form.fields[3].cursor_position = connection.username.len();

        if let Some(desc) = &connection.description {
            form.fields[4].value = desc.clone();
            form.fields[4].cursor_position = desc.len();
        }

        // Set auth method
        form.auth_method = AuthMethodSelection::from(&connection.auth_method);

        // Set tags
        form.tags_input.value = connection.tags.join(",");
        form.tags_input.cursor_position = form.tags_input.value.len();

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

    /// Move focus to the next field
    pub fn next_field(&mut self) {
        if self.auth_method_focused {
            self.auth_method_focused = false;
            self.current_field = 0;
        } else if self.current_field < self.fields.len() {
            self.current_field += 1;
        } else {
            // At tags field, wrap to first field
            self.current_field = 0;
        }
        self.update_focus();
    }

    /// Move focus to the previous field
    pub fn previous_field(&mut self) {
        if self.current_field == 0 {
            if self.auth_method_focused {
                // Wrap to tags field
                self.current_field = self.fields.len(); // Tags field
                self.auth_method_focused = false;
            } else {
                self.auth_method_focused = true;
            }
        } else if self.current_field == self.fields.len() {
            // At tags field, go to last regular field
            self.current_field = self.fields.len() - 1;
        } else {
            self.current_field -= 1;
        }
        self.update_focus();
    }

    /// Update field focus states
    fn update_focus(&mut self) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.is_focused = i == self.current_field && !self.auth_method_focused;
        }
        self.tags_input.is_focused = self.current_field == self.fields.len() && !self.auth_method_focused;
    }

    /// Select next auth method
    pub fn next_auth_method(&mut self) {
        let variants = AuthMethodSelection::variants();
        let current_index = variants.iter().position(|x| *x == self.auth_method).unwrap_or(0);
        let next_index = (current_index + 1) % variants.len();
        self.auth_method = variants[next_index].clone();
    }

    /// Select previous auth method
    pub fn previous_auth_method(&mut self) {
        let variants = AuthMethodSelection::variants();
        let current_index = variants.iter().position(|x| *x == self.auth_method).unwrap_or(0);
        let prev_index = if current_index == 0 { variants.len() - 1 } else { current_index - 1 };
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

        if self.fields[2].value.trim().is_empty() {
            errors.push("Port is required".to_string());
        } else if self.fields[2].value.parse::<u16>().is_err() {
            errors.push("Port must be a valid number (1-65535)".to_string());
        }

        if self.fields[3].value.trim().is_empty() {
            errors.push("Username is required".to_string());
        }

        errors
    }

    /// Convert form data to ServerConnection
    pub fn to_server_connection(&self) -> Result<ServerConnection, String> {
        let errors = self.validate();
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }

        let port = self.fields[2].value.parse::<u16>()
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
        
        // For public key, use custom path if different from default
        let auth_method = match &auth_config {
            AuthMethodConfig::PublicKey { .. } => {
                // TODO: Add a separate field for key path in the form
                AuthMethod::PublicKey {
                    key_path: "~/.ssh/id_rsa".to_string(),
                }
            }
            _ => auth_config.into(),
        };
        connection.auth_method = auth_method;

        // Set tags
        if !self.tags_input.value.trim().is_empty() {
            connection.tags = self.tags_input.value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // If editing, preserve the original ID
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