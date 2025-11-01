# A2A Client v0.3.0 Update TODO

This document tracks the work needed to update the a2a-client web UI to fully support the A2A Protocol v0.3.0 specification.

## Status: a2a-rs Core ✅ | a2a-client UI ✅ (Core Features Complete)

The core `a2a-rs` library has been updated to v0.3.0, including both HTTP and WebSocket clients. The web UI has been updated with core v0.3.0 features including task list, WebSocket streaming, and improved UI.

---

## 🚀 High Priority - v0.3.0 Feature Integration

### 1. Task List/History Browser ✅ COMPLETED
- [x] Add `/tasks` route to display all tasks
- [x] Implement task list view using `client.list_tasks()`
- [x] Support filtering by task state (submitted, working, completed, etc.)
- [x] Add pagination controls (cursor-based pagination)
- [ ] Support sorting options (created_at, updated_at) - Future enhancement
- [ ] Add search/filter UI for task metadata - Future enhancement
- [x] Link from task list to individual chat views
- [x] Show task summary info (status, message count, timestamps)

**API Methods:**
```rust
async fn list_tasks(&self, params: &ListTasksParams) -> Result<ListTasksResult, A2AError>
```

**Files to modify:**
- `src/bin/server.rs` - Add route handler
- `templates/tasks.html` - New template (create)
- Update navigation in `templates/index.html`

---

### 2. Push Notification Configuration Management
- [ ] Add UI to configure push notifications for tasks
- [ ] List all push notification configs for a task
- [ ] View individual config details
- [ ] Create new push notification configs
- [ ] Delete existing configs
- [ ] Support authentication types (bearer, api_key)
- [ ] Validate webhook URLs

**API Methods:**
```rust
async fn list_push_notification_configs(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>, A2AError>
async fn get_push_notification_config(&self, task_id: &str, config_id: &str) -> Result<TaskPushNotificationConfig, A2AError>
async fn delete_push_notification_config(&self, task_id: &str, config_id: &str) -> Result<(), A2AError>
async fn set_task_push_notification(...) -> Result<TaskPushNotificationConfig, A2AError>
```

**Files to modify:**
- `src/bin/server.rs` - Add CRUD route handlers
- `templates/chat.html` - Add push notification config section
- Consider separate `templates/notifications.html` for management UI

---

### 3. WebSocket Support (MAJOR UX IMPROVEMENT) ✅ COMPLETED
- [x] Add WebSocket client option alongside HTTP
- [x] Configure WebSocket endpoint in server.rs
- [x] Use `subscribe_to_task()` for real-time streaming
- [x] Remove 5-second polling, replace with SSE for browser
- [x] Handle streaming events: `StreamItem::Task`, `StreamItem::StatusUpdate`, `StreamItem::ArtifactUpdate`
- [x] Show real-time status updates in UI
- [x] Display streaming artifacts as they arrive
- [ ] Add connection status indicator - Future enhancement
- [x] Handle reconnection logic (built-in SSE reconnection)
- [x] Graceful fallback to HTTP polling if WebSocket unavailable

**API Methods:**
```rust
async fn subscribe_to_task(&self, task_id: &str) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, A2AError>>>>, A2AError>
```

**Files to modify:**
- `Cargo.toml` - Add WebSocket dependencies if needed
- `src/bin/server.rs` - WebSocket endpoint or SSE support
- Add JS for WebSocket/SSE client in templates
- Update AppState to support both HTTP and WebSocket clients

**References:**
- `a2a-rs/src/adapter/transport/websocket/client.rs` - WebSocket client implementation
- `a2a-rs/tests/client_v3_methods_test.rs` - Usage examples

---

## 🎨 Medium Priority - UI/UX Enhancements

### 4. Better Message Rendering
- [ ] Add markdown rendering for agent responses (use pulldown-cmark or similar)
- [ ] Syntax highlighting for code blocks (highlight.js or prism.js)
- [ ] Support for lists, tables, headers in markdown
- [ ] Proper escaping and sanitization
- [ ] Copy-to-clipboard for code blocks

**Dependencies to add:**
- `pulldown-cmark` (server-side) or `marked.js` (client-side)
- `highlight.js` for syntax highlighting

---

### 5. File Upload Support
- [ ] Add file input to message form
- [ ] Support multipart form data
- [ ] Convert uploaded files to FilePart
- [ ] Display file attachments in message history
- [ ] Show file metadata (name, size, mime type)
- [ ] Support multiple file uploads
- [ ] File preview for images

**Message Part Type:**
```rust
MessagePart::File(FilePart {
    file_uri: String,
    mime_type: String,
    name: Option<String>,
})
```

---

### 6. Artifact Display
- [ ] Parse and display task artifacts from responses
- [ ] Show artifact metadata (name, mime type, size)
- [ ] Render artifacts based on type (text, image, json, etc.)
- [ ] Download artifacts functionality
- [ ] Artifact preview/inline display

**Response Field:**
```rust
task.artifacts: Option<Vec<Artifact>>
```

---

### 7. Authentication Support
- [ ] Add auth token input field in UI
- [ ] Store token (session storage or cookie)
- [ ] Pass token to HTTP/WebSocket client
- [ ] Support different auth types (bearer, api_key, oauth2)
- [ ] Show authenticated status indicator
- [ ] Implement `agent/getAuthenticatedExtendedCard`

**Client Construction:**
```rust
HttpClient::with_auth(endpoint, token)
```

---

### 8. Agent Discovery & Selection
- [ ] Add agent discovery page
- [ ] Fetch and display agent cards
- [ ] Show agent capabilities, skills, providers
- [ ] Allow user to select which agent to chat with
- [ ] Display agent metadata (name, description, avatar)
- [ ] Show supported extensions (AP2, etc.)

---

### 9. Task State Visualization
- [ ] Display current task state clearly
- [ ] Show state transitions (submitted → working → completed)
- [ ] Visual indicators for each state
- [ ] Color coding for states
- [ ] Progress indicators for long-running tasks

**Task States (v0.3.0):**
- `submitted`, `working`, `input-required`, `completed`, `canceled`, `failed`, `rejected`, `auth-required`, `unknown`

---

### 10. Error Handling & Display
- [ ] Better error message display
- [ ] Show A2A protocol error codes
- [ ] Distinguish JSON-RPC errors from A2A errors
- [ ] Retry logic for failed requests
- [ ] Error recovery suggestions

**Error Codes (v0.3.0):**
- `-32001` - Task not found
- `-32002` - Task not cancelable
- `-32003` - Push notifications not supported
- `-32004` - Operation not supported
- `-32005` - Content type not supported
- `-32006` - Invalid agent response
- `-32007` - Authenticated extended card not configured

---

## 🧪 Low Priority - Testing & Refinement

### 11. Client Testing
- [ ] Integration tests for new routes
- [ ] Test task list pagination
- [ ] Test push notification CRUD
- [ ] Test WebSocket streaming (if implemented)
- [ ] Test file uploads
- [ ] Test authentication flows

---

### 12. Performance & Optimization
- [ ] Cache task list results
- [ ] Optimize polling (if keeping HTTP fallback)
- [ ] Lazy loading for large task lists
- [ ] Pagination for message history
- [ ] Asset optimization (CSS, JS minification)

---

### 13. Documentation
- [ ] Update README.md with v0.3.0 features
- [ ] Document WebSocket vs HTTP tradeoffs
- [ ] Add screenshots of new UI
- [ ] Configuration guide for push notifications
- [ ] Authentication setup guide

---

### 14. Configuration & Deployment
- [ ] Environment variable configuration
- [ ] Support multiple agent endpoints
- [ ] Docker/container support
- [ ] Production build optimizations
- [ ] HTTPS/TLS configuration

---

## 📋 API Method Coverage Checklist

### ✅ Already Implemented (v0.2.x)
- [x] `send_raw_request()` - Raw JSON-RPC requests
- [x] `send_request()` - Structured requests
- [x] `send_task_message()` - Send user messages
- [x] `get_task()` - Retrieve task state
- [x] `cancel_task()` - Cancel task
- [x] `set_task_push_notification()` - Setup push notifications
- [x] `get_task_push_notification()` - Get notification config

### 🆕 New in v0.3.0 (Not Yet Used in UI)
- [ ] `list_tasks()` - List/filter tasks with pagination
- [ ] `list_push_notification_configs()` - List all configs for a task
- [ ] `get_push_notification_config()` - Get specific config by ID
- [ ] `delete_push_notification_config()` - Delete config by ID
- [ ] `subscribe_to_task()` - Stream task updates (WebSocket only)

### 🔐 Agent Discovery (Not Yet Implemented)
- [ ] `agent/getAuthenticatedExtendedCard` - Get extended agent card with auth

---

## 📁 Files That Need Modification

### Core Server
- [ ] `src/bin/server.rs` - Add routes, WebSocket support, state management

### Templates
- [ ] `templates/index.html` - Update with links to new pages
- [ ] `templates/chat.html` - Add push notification UI, better rendering
- [ ] `templates/tasks.html` - NEW: Task list/history browser
- [ ] `templates/notifications.html` - NEW: Push notification management (optional)
- [ ] `templates/agent.html` - NEW: Agent discovery (optional)

### Assets
- [ ] `src/styles.css` - Style updates for new components

### Configuration
- [ ] `Cargo.toml` - Add dependencies (markdown, WebSocket, etc.)
- [ ] `README.md` - Update documentation

---

## 🔗 Reference Files

**Core Library (Already Updated):**
- `a2a-rs/src/services/client.rs` - Client trait definition
- `a2a-rs/src/adapter/transport/http/client.rs` - HTTP client implementation
- `a2a-rs/src/adapter/transport/websocket/client.rs` - WebSocket client implementation

**Test Examples:**
- `a2a-rs/tests/client_v3_methods_test.rs` - v0.3.0 method usage examples
- `a2a-rs/tests/spec_compliance_test.rs` - Protocol compliance tests

**Specification:**
- `spec/README.md` - A2A Protocol v0.3.0 overview
- `spec/requests.json` - Method definitions
- `spec/notifications.json` - Push notification schema

---

## 🎯 Recommended Implementation Order

1. **Phase 1 - Core v0.3.0 Features**
   - Task list/history browser (#1)
   - Push notification management UI (#2)

2. **Phase 2 - Real-time Experience**
   - WebSocket support (#3)
   - Better message rendering (#4)

3. **Phase 3 - Enhanced Features**
   - Authentication support (#7)
   - File upload support (#5)
   - Artifact display (#6)

4. **Phase 4 - Polish**
   - Agent discovery (#8)
   - Error handling improvements (#10)
   - Testing (#11)
   - Documentation updates (#13)

---

## 💡 Notes

- The core `a2a-rs` library is fully v0.3.0 compliant
- Both HTTP and WebSocket clients are ready to use
- The web UI will continue to work with v0.3.0 backend (backward compatible)
- WebSocket support will provide the biggest UX improvement (eliminates polling)
- See `a2a-client/TODO.md` for additional future feature ideas

---

Last Updated: 2025-10-23
Protocol Version: v0.3.0
