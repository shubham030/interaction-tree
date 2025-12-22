# Debug Agent Test App

This example app demonstrates the Interaction Tree package and provides **intentional bugs** for testing the Debug Agent.

## Getting Started

```bash
# From the example directory
flutter pub get
flutter run
```

## Debug Scenarios

The app contains intentional bugs that the Debug Agent should be able to find and diagnose.

### 1. 🐛 Login Timeout Bug

**How to trigger:**
1. Open the debug menu (🪲 icon in login screen appbar)
2. Enable "Simulate Timeout"
3. Try to login

**Expected behavior:** App should show an error message
**Actual behavior:** App crashes with unhandled `NetworkTimeoutException`

**Root cause:**
- Location: `lib/state/app_state.dart:AuthState.login()`
- The `try-catch` only catches `ApiException` but not `NetworkTimeoutException`

---

### 2. 🐛 Cart Remove Bug

**How to trigger:**
1. Login to the app
2. Go to Shop tab
3. Add some items to cart
4. Go to Cart tab
5. Tap the remove (X) button on an item

**Expected behavior:** Item should be removed and UI should update
**Actual behavior:** Item is removed from data but UI doesn't refresh

**Root cause:**
- Location: `lib/state/app_state.dart:CartState.removeItem()`
- Missing `notifyListeners()` call after removing the item

---

### 3. 🐛 Empty Cart Checkout Bug

**How to trigger:**
1. Login to the app
2. Go to Cart tab with an empty cart
3. Notice the "Proceed to Checkout" button is still enabled
4. Tap checkout and proceed through the flow

**Expected behavior:** Checkout button should be disabled when cart is empty
**Actual behavior:** Button is always enabled, allows checkout with empty cart

**Root cause:**
- Location: `lib/screens/cart_screen.dart` - checkout button
- Location: `lib/screens/checkout_screen.dart` - continue buttons
- Missing cart empty validation before allowing navigation

---

### 4. 🐛 Cart Race Condition

**How to trigger:**
1. Login to the app
2. Go to Shop tab
3. Tap "Add All" button rapidly multiple times

**Expected behavior:** Cart total should be consistent with items
**Actual behavior:** Cart total can become inconsistent due to race conditions

**Root cause:**
- Location: `lib/state/app_state.dart:CartState.addItem()`
- Location: `lib/services/api_service.dart:addToCart()`
- Optimistic updates without proper version checking

---

### 5. 🐛 Unauthenticated Checkout

**How to trigger:**
1. Login normally
2. Add items to cart
3. Start checkout flow
4. (In a real scenario: token expires mid-checkout)
5. Try to complete payment

**Expected behavior:** Should validate auth before processing payment
**Actual behavior:** Payment form allows submission without auth check

**Root cause:**
- Location: `lib/screens/checkout_screen.dart:_processPayment()`
- Location: `lib/services/api_service.dart:processCheckout()`
- Missing auth validation before processing

---

## Debug Agent Testing

To test the Debug Agent with this app:

1. Start the app:
   ```bash
   flutter run
   ```

2. Start the Fleeter daemon:
   ```bash
   cd packages/fleeter-daemon
   pnpm run dev
   ```

3. Create a session and connect:
   ```bash
   # Use the MCP tools or CLI to create a session
   ```

4. Ask the Debug Agent to investigate:
   ```
   "Debug the cart - removing items doesn't update the UI"
   ```
   
   ```
   "Debug the checkout flow - it allows checkout with an empty cart"
   ```
   
   ```
   "Debug the login - it crashes when there's a network timeout"
   ```

## App Structure

```
lib/
├── main.dart              # App entry, navigation, debug scenario cards
├── services/
│   └── api_service.dart   # Simulated API with configurable failures
├── state/
│   └── app_state.dart     # Auth, Cart, Checkout state (with bugs)
└── screens/
    ├── login_screen.dart    # Login with debug controls
    ├── shop_screen.dart     # Product grid, add to cart
    ├── cart_screen.dart     # Cart list, remove items
    └── checkout_screen.dart # Multi-step checkout flow
```

## Interaction Keys

All interactive widgets have `InteractionKey` for the agent to find and interact with them:

| Key | Widget | Description |
|-----|--------|-------------|
| `login-btn` | FilledButton | Submit login form |
| `email-input` | TextFormField | Email input |
| `password-input` | TextFormField | Password input |
| `debug-menu-btn` | PopupMenuButton | Debug failure mode toggle |
| `cart-icon-btn` | IconButton | Open cart |
| `add-to-cart-btn-{id}` | FilledButton | Add product to cart |
| `remove-item-{id}` | IconButton | Remove item from cart |
| `checkout-btn` | FilledButton | Proceed to checkout |
| `place-order-btn` | FilledButton | Submit payment |
| ... | ... | See code for full list |

## Expected Debug Reports

When the Debug Agent investigates these bugs, it should produce reports like:

```markdown
## 🔍 Debug Report: Cart Remove Not Updating UI

### Summary
Removing items from cart doesn't trigger UI update because 
`CartState.removeItem()` doesn't call `notifyListeners()`.

### Errors Found
| Error | Location | Context |
|-------|----------|---------|
| Missing notifyListeners | `app_state.dart:142` | After removeWhere |

### Root Cause Analysis
1. User taps remove button on cart item
2. `CartState.removeItem()` is called
3. Item is removed from `_items` list
4. `notifyListeners()` is NOT called
5. UI never rebuilds to show updated state

### Code Locations
**Primary issue**:
- File: `lib/state/app_state.dart`
- Line: 142
- Method: `removeItem()`
- Problem: Missing `notifyListeners()` after removal

### Suggested Fix
Add `notifyListeners()` after the `_items.removeWhere()` call.

**Files to modify**:
1. `lib/state/app_state.dart` - add notifyListeners() in removeItem()
```
