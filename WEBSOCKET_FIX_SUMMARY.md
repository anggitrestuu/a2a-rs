# WebSocket "Task Not Found" Fix - Complete Summary

## Problem

When starting a new chat, the client would immediately subscribe to a WebSocket stream for a task that didn't exist yet, causing:

```
ERROR: Stream error: Task not found
ERROR: WebSocket protocol error: Connection reset without closing handshake
```

## Root Causes

### 1. **Request Processor** (a2a-rs/src/adapter/business/request_processor.rs:173-209)
- `tasks/resubscribe` would fail immediately if task didn't exist
- **Fix**: Return `null` result for non-existent tasks instead of error

### 2. **Storage Layer** (task_storage.rs:508-514, sqlx_storage.rs:1036-1042)
- `add_status_subscriber()` tried to fetch task to send initial update
- Would fail with "Task not found" for new subscriptions
- **Fix**: Use `if let Ok(task)` to gracefully skip initial update if task doesn't exist

### 3. **WebSocket Client** (websocket/client.rs:491-615)
- Stream would fail on `null` response from server
- **Fix**: Loop inside unfold to skip null messages and wait for task creation

## Changes Made

### 1. DefaultRequestProcessor - Allow Null Responses
```rust
// Before: Failed immediately
let task = self.task_manager.get_task(&params.id, params.history_length).await?;

// After: Return null if task doesn't exist
match self.task_manager.get_task(...).await {
    Ok(task) => Ok(JSONRPCResponse::success(...)),
    Err(A2AError::TaskNotFound(_)) => Ok(JSONRPCResponse::success(id, Value::Null)),
    Err(e) => Err(e),
}
```

### 2. Storage - Graceful Subscriber Addition
```rust
// Before: Failed if task didn't exist
let task = self.get_task(task_id, None).await?;
self.broadcast_status_update(task_id, task.status, false).await?;

// After: Skip initial broadcast if task doesn't exist
if let Ok(task) = self.get_task(task_id, None).await {
    let _ = self.broadcast_status_update(task_id, task.status, false).await;
}
```

### 3. WebSocket Client - Skip Null Messages
```rust
// Added loop to retry on null responses
loop {
    let message = get_next_websocket_message().await;

    if result.is_null() {
        debug!("Task doesn't exist yet, waiting for next message");
        continue; // Skip and wait for next message
    }

    // Parse and return actual updates
    return Some((stream_item, conn));
}
```

## New Flow

**Before:**
1. Start new chat → Generate task ID
2. Subscribe to WebSocket → **ERROR: Task not found**
3. Connection closes
4. User confused

**After:**
1. Start new chat → Generate task ID
2. Subscribe to WebSocket → **SUCCESS** (returns `null`)
3. Client waits for first message
4. User sends message → Task created
5. WebSocket receives real-time updates ✅

## Testing

### Terminal 1: Start Agent with Tracing
```cmd
C:\Users\EmilLindfors\dev\a2a-rs> run-with-tracing.bat
```

### Terminal 2: Start Client with Tracing
```cmd
C:\Users\EmilLindfors\dev\a2a-rs> run-client-with-tracing.bat
```

### Expected Log Output
```
INFO Using WebSocket client at ws://localhost:8081
INFO Server listening on http://127.0.0.1:3000
INFO Attempting to subscribe to task <uuid> via WebSocket
INFO Successfully subscribed to task <uuid> via WebSocket
DEBUG Task doesn't exist yet, waiting for next message
[User sends first message]
INFO Parsed streaming response as Task
INFO Parsed streaming response as StatusUpdate
```

## Architecture Insights

### Chat vs Task Confusion

**The Issue:**
- Current client treats "New Chat" = "New Task"
- A2A Protocol: **Task = Specific Job** (e.g., one reimbursement request)
- This creates UX confusion

**Better Approach for Showcase:**

#### Option A: Task-Per-Request (Recommended)
```
Landing Page
  ↓
[Submit Expense] button
  ↓
Task Created = Reimbursement Request
  - Task ID = Reimbursement ID
  - States: submitted → working → completed
  - Messages = Conversation about THIS expense
```

#### Option B: Immediate Task Creation (Quick Fix)
```
New Chat → Send placeholder message
        ↓
  Task exists immediately
  WebSocket works from the start
```

### Showcase Reimbursement Agent Ideas

**1. Better Landing Page**
- "Submit $50 lunch expense"
- "Track reimbursement #1234"
- "Upload receipt"

**2. Task List Integration**
```
/tasks → Shows all reimbursements
  - Filter by state (pending, approved, paid)
  - Click to view/continue conversation
```

**3. Rich Content Types**
- **Text**: "I need to get reimbursed for lunch"
- **Data**: Structured JSON with amount, category, date
- **Files**: Receipt uploads
- **Artifacts**: Generated PDFs, forms

**4. v0.3.0 Feature Showcase**
- ✅ Task listing and filtering
- ✅ Push notifications on approval
- ✅ Streaming for real-time form generation
- ✅ State transition history
- ✅ Authenticated extended cards

## Next Steps

### Immediate
- [ ] Test new chat flow with WebSocket
- [ ] Verify no more "Task not found" errors
- [ ] Check real-time updates work

### UX Improvements
- [ ] Redesign landing page for reimbursement-first UX
- [ ] Add task list view
- [ ] Show reimbursement lifecycle clearly
- [ ] Add example requests/quick actions

### Feature Showcase
- [ ] Implement structured data messages
- [ ] Add file upload support
- [ ] Generate PDF artifacts
- [ ] Set up push notification demo
- [ ] Show state transitions in UI

## Files Modified

1. **a2a-rs/src/adapter/business/request_processor.rs**
2. **a2a-rs/src/adapter/storage/task_storage.rs**
3. **a2a-rs/src/adapter/storage/sqlx_storage.rs**
4. **a2a-rs/src/adapter/transport/websocket/client.rs**
5. **a2a-client/src/bin/client.rs** (retry logic)

## Tracing Helper Scripts Created

- **run-with-tracing.bat** - Run agent with DEBUG logging
- **run-client-with-tracing.bat** - Run client with DEBUG logging
- **run-with-tracing.sh** - Unix version

---

**Status**: ✅ All fixes implemented and building successfully

**Last Updated**: 2025-10-24
