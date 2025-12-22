/// Shop screen for adding items to cart.
/// Used to test cart state management and debug race conditions.

import 'package:flutter/material.dart';
import 'package:interaction_tree/interaction_tree.dart';
import '../state/app_state.dart';

class ShopScreen extends StatelessWidget {
  final CartState cartState;
  final VoidCallback onViewCart;
  
  const ShopScreen({
    super.key,
    required this.cartState,
    required this.onViewCart,
  });
  
  // Sample products
  static final List<Map<String, dynamic>> _products = [
    {'id': 'prod-1', 'name': 'Widget Pro', 'price': 29.99, 'icon': Icons.widgets},
    {'id': 'prod-2', 'name': 'Flutter Kit', 'price': 49.99, 'icon': Icons.flutter_dash},
    {'id': 'prod-3', 'name': 'Dart Tools', 'price': 19.99, 'icon': Icons.build},
    {'id': 'prod-4', 'name': 'Debug Helper', 'price': 39.99, 'icon': Icons.bug_report},
    {'id': 'prod-5', 'name': 'State Manager', 'price': 59.99, 'icon': Icons.account_tree},
    {'id': 'prod-6', 'name': 'UI Components', 'price': 24.99, 'icon': Icons.dashboard},
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Shop'),
        actions: [
          ListenableBuilder(
            listenable: cartState,
            builder: (context, _) {
              return Stack(
                alignment: Alignment.center,
                children: [
                  IconButton(
                    key: const InteractionKey(
                      'cart-icon-btn',
                      description: 'Open shopping cart',
                    ),
                    icon: const Icon(Icons.shopping_cart),
                    onPressed: onViewCart,
                  ),
                  if (cartState.itemCount > 0)
                    Positioned(
                      right: 8,
                      top: 8,
                      child: Container(
                        key: const InteractionKey(
                          'cart-badge',
                          description: 'Cart item count badge',
                        ),
                        padding: const EdgeInsets.all(4),
                        decoration: const BoxDecoration(
                          color: Colors.red,
                          shape: BoxShape.circle,
                        ),
                        child: Text(
                          '${cartState.itemCount}',
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ),
                ],
              );
            },
          ),
        ],
      ),
      body: ListenableBuilder(
        listenable: cartState,
        builder: (context, _) {
          return GridView.builder(
            key: const InteractionKey(
              'products-grid',
              description: 'Grid of products',
            ),
            padding: const EdgeInsets.all(16),
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              childAspectRatio: 0.75,
              crossAxisSpacing: 16,
              mainAxisSpacing: 16,
            ),
            itemCount: _products.length,
            itemBuilder: (context, index) {
              final product = _products[index];
              final inCart = cartState.items.any(
                (item) => item.productId == product['id'],
              );
              
              return Card(
                key: InteractionKey(
                  'product-card-${product['id']}',
                  description: 'Product: ${product['name']}',
                ),
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // Product icon
                      Expanded(
                        child: Container(
                          decoration: BoxDecoration(
                            color: Colors.grey[100],
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Icon(
                            product['icon'] as IconData,
                            size: 48,
                            color: Colors.deepPurple,
                          ),
                        ),
                      ),
                      const SizedBox(height: 8),
                      
                      // Product name
                      Text(
                        product['name'] as String,
                        style: Theme.of(context).textTheme.titleMedium,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      
                      // Price
                      Text(
                        '\$${(product['price'] as double).toStringAsFixed(2)}',
                        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                          color: Colors.deepPurple,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      
                      const SizedBox(height: 8),
                      
                      // Add to cart button
                      SizedBox(
                        height: 36,
                        child: inCart
                          ? OutlinedButton.icon(
                              key: InteractionKey(
                                'in-cart-btn-${product['id']}',
                                description: '${product['name']} already in cart',
                              ),
                              onPressed: onViewCart,
                              icon: const Icon(Icons.check, size: 16),
                              label: const Text('In Cart'),
                            )
                          : FilledButton.icon(
                              key: InteractionKey(
                                'add-to-cart-btn-${product['id']}',
                                description: 'Add ${product['name']} to cart',
                              ),
                              onPressed: cartState.isLoading
                                ? null
                                : () => _addToCart(product),
                              icon: const Icon(Icons.add_shopping_cart, size: 16),
                              label: const Text('Add'),
                            ),
                      ),
                    ],
                  ),
                ),
              );
            },
          );
        },
      ),
      floatingActionButton: FloatingActionButton.extended(
        key: const InteractionKey(
          'add-all-fab',
          description: 'Add all products to cart (for testing)',
        ),
        onPressed: () => _addAllToCart(),
        icon: const Icon(Icons.add_shopping_cart),
        label: const Text('Add All'),
      ),
    );
  }
  
  void _addToCart(Map<String, dynamic> product) {
    cartState.addItem(
      product['id'] as String,
      product['name'] as String,
      product['price'] as double,
    );
  }
  
  /// Add all products rapidly to test race conditions
  void _addAllToCart() {
    for (final product in _products) {
      cartState.addItem(
        product['id'] as String,
        product['name'] as String,
        product['price'] as double,
      );
    }
  }
}
