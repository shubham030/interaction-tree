/// Cart screen for Debug Agent testing.

import 'package:flutter/material.dart';
import 'package:interaction_tree/interaction_tree.dart';
import '../state/app_state.dart';

class CartScreen extends StatelessWidget {
  final CartState cartState;
  final VoidCallback onCheckout;
  final VoidCallback onContinueShopping;
  
  const CartScreen({
    super.key,
    required this.cartState,
    required this.onCheckout,
    required this.onContinueShopping,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Shopping Cart'),
        actions: [
          IconButton(
            key: const InteractionKey(
              'refresh-cart-btn',
              description: 'Refresh cart from server',
            ),
            icon: const Icon(Icons.refresh),
            onPressed: () => cartState.refresh(),
          ),
          IconButton(
            key: const InteractionKey(
              'clear-cart-btn',
              description: 'Clear all items from cart',
            ),
            icon: const Icon(Icons.delete_sweep),
            onPressed: cartState.isEmpty ? null : () => _confirmClearCart(context),
          ),
        ],
      ),
      body: ListenableBuilder(
        listenable: cartState,
        builder: (context, _) {
          if (cartState.isLoading) {
            return const Center(
              child: CircularProgressIndicator(),
            );
          }
          
          if (cartState.isEmpty) {
            return _buildEmptyCart(context);
          }
          
          return _buildCartList(context);
        },
      ),
      bottomNavigationBar: ListenableBuilder(
        listenable: cartState,
        builder: (context, _) {
          return _buildBottomBar(context);
        },
      ),
    );
  }
  
  Widget _buildEmptyCart(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.shopping_cart_outlined,
            size: 80,
            color: Colors.grey[400],
          ),
          const SizedBox(height: 16),
          Text(
            'Your cart is empty',
            key: const InteractionKey(
              'empty-cart-message',
              description: 'Empty cart message',
            ),
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
              color: Colors.grey[600],
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Add some items to get started',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Colors.grey[500],
            ),
          ),
          const SizedBox(height: 24),
          FilledButton.icon(
            key: const InteractionKey(
              'continue-shopping-btn',
              description: 'Navigate to shop',
            ),
            onPressed: onContinueShopping,
            icon: const Icon(Icons.shopping_bag),
            label: const Text('Continue Shopping'),
          ),
        ],
      ),
    );
  }
  
  Widget _buildCartList(BuildContext context) {
    return ListView.builder(
      key: const InteractionKey(
        'cart-items-list',
        description: 'List of cart items',
      ),
      padding: const EdgeInsets.all(16),
      itemCount: cartState.items.length,
      itemBuilder: (context, index) {
        final item = cartState.items[index];
        return Card(
          key: InteractionKey(
            'cart-item-${item.productId}',
            description: 'Cart item: ${item.name}',
          ),
          margin: const EdgeInsets.only(bottom: 12),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                // Product image placeholder
                Container(
                  width: 60,
                  height: 60,
                  decoration: BoxDecoration(
                    color: Colors.grey[200],
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Icon(Icons.image, color: Colors.grey),
                ),
                const SizedBox(width: 12),
                
                // Product details
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        item.name,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        '\$${item.price.toStringAsFixed(2)}',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: Colors.deepPurple,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
                
                // Quantity controls
                Row(
                  children: [
                    IconButton(
                      key: InteractionKey(
                        'decrease-qty-${item.productId}',
                        description: 'Decrease quantity of ${item.name}',
                      ),
                      icon: const Icon(Icons.remove_circle_outline),
                      onPressed: item.quantity > 1
                        ? () => _updateQuantity(item, -1)
                        : null,
                    ),
                    Text(
                      '${item.quantity}',
                      key: InteractionKey(
                        'qty-display-${item.productId}',
                        description: 'Quantity of ${item.name}',
                      ),
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    IconButton(
                      key: InteractionKey(
                        'increase-qty-${item.productId}',
                        description: 'Increase quantity of ${item.name}',
                      ),
                      icon: const Icon(Icons.add_circle_outline),
                      onPressed: () => _updateQuantity(item, 1),
                    ),
                  ],
                ),
                
                // Remove button
                IconButton(
                  key: InteractionKey(
                    'remove-item-${item.productId}',
                    description: 'Remove ${item.name} from cart',
                  ),
                  icon: const Icon(Icons.delete_outline, color: Colors.red),
                  onPressed: () => _removeItem(item),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
  
  Widget _buildBottomBar(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white,
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.1),
            blurRadius: 8,
            offset: const Offset(0, -2),
          ),
        ],
      ),
      child: SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Cart summary
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Items: ${cartState.itemCount}',
                  key: const InteractionKey(
                    'cart-item-count',
                    description: 'Total number of items in cart',
                  ),
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                Text(
                  'Total: \$${cartState.total.toStringAsFixed(2)}',
                  key: const InteractionKey(
                    'cart-total',
                    description: 'Total cart value',
                  ),
                  style: Theme.of(context).textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            
            // Checkout button
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                key: const InteractionKey(
                  'checkout-btn',
                  description: 'Proceed to checkout',
                ),
                onPressed: cartState.isEmpty ? onCheckout : onCheckout,
                icon: const Icon(Icons.shopping_cart_checkout),
                label: const Text('Proceed to Checkout'),
              ),
            ),
            
            // Error display
            if (cartState.error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  cartState.error!,
                  key: const InteractionKey(
                    'cart-error',
                    description: 'Cart error message',
                  ),
                  style: const TextStyle(color: Colors.red),
                ),
              ),
              
            // Debug info
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(
                'Debug: items=${cartState.items.length}, loading=${cartState.isLoading}',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Colors.grey,
                  fontFamily: 'monospace',
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
  
  void _updateQuantity(CartItem item, int delta) {
    final newQty = item.quantity + delta;
    cartState.updateQuantity(item.productId, newQty);
  }
  
  void _removeItem(CartItem item) {
    cartState.removeItem(item.productId);
  }
  
  void _confirmClearCart(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Cart?'),
        content: const Text('Are you sure you want to remove all items from your cart?'),
        actions: [
          TextButton(
            key: const InteractionKey(
              'cancel-clear-btn',
              description: 'Cancel clear cart',
            ),
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const InteractionKey(
              'confirm-clear-btn',
              description: 'Confirm clear cart',
            ),
            onPressed: () {
              Navigator.pop(context);
              cartState.clear();
            },
            child: const Text('Clear'),
          ),
        ],
      ),
    );
  }
}
