/// Interaction Tree Example App

import 'package:flutter/material.dart';
import 'package:interaction_tree/interaction_tree.dart';

import 'state/app_state.dart';
import 'screens/login_screen.dart';
import 'screens/shop_screen.dart';
import 'screens/cart_screen.dart';
import 'screens/checkout_screen.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  // Register VM service extensions for LLM/MCP interaction
  InteractionTreeService.ensureInitialized();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Debug Agent Test App',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
        useMaterial3: true,
      ),
      home: const AppShell(),
    );
  }
}

/// Main app shell that manages navigation and state
class AppShell extends StatefulWidget {
  const AppShell({super.key});

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  // App state
  final _authState = AuthState();
  final _cartState = CartState();
  final _checkoutState = CheckoutState();
  
  // Navigation
  int _currentIndex = 0;
  bool _showCheckout = false;
  
  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: _authState,
      builder: (context, _) {
        // Show login screen if not authenticated
        if (!_authState.isAuthenticated) {
          return LoginScreen(
            authState: _authState,
            onLoginSuccess: () {
              setState(() => _currentIndex = 0);
            },
          );
        }
        
        // Show checkout flow if active
        if (_showCheckout) {
          return CheckoutScreen(
            checkoutState: _checkoutState,
            cartState: _cartState,
            authState: _authState,
            onOrderComplete: () {
              setState(() {
                _showCheckout = false;
                _currentIndex = 0;
              });
            },
            onBack: () {
              setState(() => _showCheckout = false);
            },
          );
        }
        
        // Main app with bottom navigation
        return Scaffold(
          body: IndexedStack(
            index: _currentIndex,
            children: [
              // Home tab - original demo content
              _buildHomeTab(),
              
              // Shop tab
              ShopScreen(
                cartState: _cartState,
                onViewCart: () => setState(() => _currentIndex = 2),
              ),
              
              // Cart tab
              CartScreen(
                cartState: _cartState,
                onCheckout: () => setState(() => _showCheckout = true),
                onContinueShopping: () => setState(() => _currentIndex = 1),
              ),
            ],
          ),
          bottomNavigationBar: NavigationBar(
            key: const InteractionKey(
              'bottom-nav',
              description: 'Main bottom navigation bar',
            ),
            selectedIndex: _currentIndex,
            onDestinationSelected: (index) {
              setState(() => _currentIndex = index);
            },
            destinations: [
              const NavigationDestination(
                key: InteractionKey('nav-home', description: 'Home tab'),
                icon: Icon(Icons.home_outlined),
                selectedIcon: Icon(Icons.home),
                label: 'Home',
              ),
              const NavigationDestination(
                key: InteractionKey('nav-shop', description: 'Shop tab'),
                icon: Icon(Icons.store_outlined),
                selectedIcon: Icon(Icons.store),
                label: 'Shop',
              ),
              NavigationDestination(
                key: const InteractionKey('nav-cart', description: 'Cart tab'),
                icon: Badge(
                  label: Text('${_cartState.itemCount}'),
                  isLabelVisible: _cartState.itemCount > 0,
                  child: const Icon(Icons.shopping_cart_outlined),
                ),
                selectedIcon: Badge(
                  label: Text('${_cartState.itemCount}'),
                  isLabelVisible: _cartState.itemCount > 0,
                  child: const Icon(Icons.shopping_cart),
                ),
                label: 'Cart',
              ),
            ],
          ),
        );
      },
    );
  }
  
  Widget _buildHomeTab() {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('Debug Agent Test App'),
        actions: [
          IconButton(
            key: const InteractionKey(
              'logout-btn',
              description: 'Log out current user',
            ),
            icon: const Icon(Icons.logout),
            onPressed: () => _authState.logout(),
          ),
        ],
      ),
      body: ListView(
        key: const InteractionKey(
          'home-list',
          description: 'Home screen content list',
        ),
        padding: const EdgeInsets.all(16),
        children: [
          // User info card
          Card(
            child: ListTile(
              key: const InteractionKey(
                'user-info-tile',
                description: 'Current user information',
              ),
              leading: const CircleAvatar(
                child: Icon(Icons.person),
              ),
              title: Text(_authState.userName ?? 'User'),
              subtitle: Text(_authState.userEmail ?? ''),
              trailing: const Icon(Icons.chevron_right),
            ),
          ),
          
          const SizedBox(height: 16),
          
          // Quick actions
          Text(
            'Quick Actions',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          
          _buildScenarioCard(
            key: 'action-add-to-cart',
            title: '🛒 Add Test Item',
            description: 'Add a test item to cart and go to cart screen.',
            action: 'Add item → View cart',
            onTap: () {
              _cartState.addItem('test-1', 'Test Item', 9.99);
              setState(() => _currentIndex = 2);
            },
          ),
          
          _buildScenarioCard(
            key: 'action-view-cart',
            title: '🛍️ View Cart',
            description: 'Go to the cart screen.',
            action: 'Open cart',
            onTap: () {
              setState(() => _currentIndex = 2);
            },
          ),
          
          _buildScenarioCard(
            key: 'action-shop',
            title: '🏪 Go Shopping',
            description: 'Browse products and add to cart.',
            action: 'Open shop',
            onTap: () => setState(() => _currentIndex = 1),
          ),
          
          const SizedBox(height: 24),
          
          // Demo section
          Text(
            'Demo',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          
          // Counter card
          InteractionContext(
            name: 'counter-section',
            description: 'Counter demo section',
            child: _CounterCard(),
          ),
          
          const SizedBox(height: 16),
          
          // Expandable section
          const _ExpandableCard(),
        ],
      ),
    );
  }
  
  Widget _buildScenarioCard({
    required String key,
    required String title,
    required String description,
    required String action,
    required VoidCallback onTap,
  }) {
    return Card(
      key: InteractionKey(key, description: title),
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              Text(
                description,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Colors.grey[600],
                ),
              ),
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: Colors.deepPurple[50],
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  action,
                  style: TextStyle(
                    color: Colors.deepPurple[700],
                    fontSize: 12,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Simple counter card (from original demo)
class _CounterCard extends StatefulWidget {
  @override
  State<_CounterCard> createState() => _CounterCardState();
}

class _CounterCardState extends State<_CounterCard> {
  int _counter = 0;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Text(
              'Counter: $_counter',
              key: const InteractionKey(
                'counter-display',
                description: 'Current counter value',
              ),
              style: Theme.of(context).textTheme.headlineMedium,
            ),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                FilledButton.icon(
                  key: const InteractionKey(
                    'decrement-btn',
                    description: 'Decrease counter',
                  ),
                  onPressed: () => setState(() => _counter--),
                  icon: const Icon(Icons.remove),
                  label: const Text('Decrease'),
                ),
                const SizedBox(width: 16),
                FilledButton.icon(
                  key: const InteractionKey(
                    'increment-btn',
                    description: 'Increase counter',
                  ),
                  onPressed: () => setState(() => _counter++),
                  icon: const Icon(Icons.add),
                  label: const Text('Increase'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// Expandable card (from original demo)
class _ExpandableCard extends StatefulWidget {
  const _ExpandableCard();

  @override
  State<_ExpandableCard> createState() => _ExpandableCardState();
}

class _ExpandableCardState extends State<_ExpandableCard> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Column(
        children: [
          ListTile(
            key: const InteractionKey(
              'expandable-header',
              description: 'Toggle expandable section',
            ),
            title: const Text('Expandable Section'),
            subtitle: Text(_expanded ? 'Tap to collapse' : 'Tap to expand'),
            trailing: Icon(_expanded ? Icons.expand_less : Icons.expand_more),
            onTap: () => setState(() => _expanded = !_expanded),
          ),
          AnimatedCrossFade(
            firstChild: const SizedBox.shrink(),
            secondChild: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text(
                    'This content appears when expanded.',
                  ),
                  const SizedBox(height: 12),
                  FilledButton.tonal(
                    key: const InteractionKey(
                      'expanded-action-btn',
                      description: 'Action inside expanded section',
                    ),
                    onPressed: () {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(
                          content: Text('Action from expanded section!'),
                        ),
                      );
                    },
                    child: const Text('Expanded Action'),
                  ),
                ],
              ),
            ),
            crossFadeState: _expanded
                ? CrossFadeState.showSecond
                : CrossFadeState.showFirst,
            duration: const Duration(milliseconds: 200),
          ),
        ],
      ),
    );
  }
}
