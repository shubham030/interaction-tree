/// Checkout screen for Debug Agent testing.

import 'package:flutter/material.dart';
import 'package:interaction_tree/interaction_tree.dart';
import '../state/app_state.dart';

class CheckoutScreen extends StatefulWidget {
  final CheckoutState checkoutState;
  final CartState cartState;
  final AuthState authState;
  final VoidCallback onOrderComplete;
  final VoidCallback onBack;
  
  const CheckoutScreen({
    super.key,
    required this.checkoutState,
    required this.cartState,
    required this.authState,
    required this.onOrderComplete,
    required this.onBack,
  });

  @override
  State<CheckoutScreen> createState() => _CheckoutScreenState();
}

class _CheckoutScreenState extends State<CheckoutScreen> {
  final _formKey = GlobalKey<FormState>();
  
  // Shipping form controllers
  final _nameController = TextEditingController();
  final _addressController = TextEditingController();
  final _cityController = TextEditingController();
  final _zipController = TextEditingController();
  
  // Payment form controllers
  final _cardNumberController = TextEditingController();
  final _expiryController = TextEditingController();
  final _cvvController = TextEditingController();
  
  @override
  void dispose() {
    _nameController.dispose();
    _addressController.dispose();
    _cityController.dispose();
    _zipController.dispose();
    _cardNumberController.dispose();
    _expiryController.dispose();
    _cvvController.dispose();
    super.dispose();
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          key: const InteractionKey(
            'checkout-back-btn',
            description: 'Go back from checkout',
          ),
          icon: const Icon(Icons.arrow_back),
          onPressed: widget.onBack,
        ),
        title: const Text('Checkout'),
      ),
      body: ListenableBuilder(
        listenable: Listenable.merge([
          widget.checkoutState,
          widget.cartState,
        ]),
        builder: (context, _) {
          return Column(
            children: [
              // Stepper indicator
              _buildStepIndicator(context),
              
              // Step content
              Expanded(
                child: _buildStepContent(context),
              ),
            ],
          );
        },
      ),
    );
  }
  
  Widget _buildStepIndicator(BuildContext context) {
    final currentStep = widget.checkoutState.currentStep;
    
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 16),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: CheckoutStep.values.map((step) {
          final isActive = step.index <= currentStep.index;
          final isCurrent = step == currentStep;
          
          return Column(
            children: [
              Container(
                key: InteractionKey(
                  'step-indicator-${step.name}',
                  description: 'Checkout step: ${step.name}',
                ),
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: isActive ? Colors.deepPurple : Colors.grey[300],
                  border: isCurrent 
                    ? Border.all(color: Colors.deepPurple, width: 3)
                    : null,
                ),
                child: Center(
                  child: Text(
                    '${step.index + 1}',
                    style: TextStyle(
                      color: isActive ? Colors.white : Colors.grey[600],
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 4),
              Text(
                _stepLabel(step),
                style: TextStyle(
                  fontSize: 12,
                  color: isActive ? Colors.deepPurple : Colors.grey[600],
                ),
              ),
            ],
          );
        }).toList(),
      ),
    );
  }
  
  String _stepLabel(CheckoutStep step) {
    switch (step) {
      case CheckoutStep.cart:
        return 'Cart';
      case CheckoutStep.shipping:
        return 'Shipping';
      case CheckoutStep.payment:
        return 'Payment';
      case CheckoutStep.confirmation:
        return 'Done';
    }
  }
  
  Widget _buildStepContent(BuildContext context) {
    switch (widget.checkoutState.currentStep) {
      case CheckoutStep.cart:
        return _buildCartReview(context);
      case CheckoutStep.shipping:
        return _buildShippingForm(context);
      case CheckoutStep.payment:
        return _buildPaymentForm(context);
      case CheckoutStep.confirmation:
        return _buildConfirmation(context);
    }
  }
  
  Widget _buildCartReview(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Review Your Cart',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 16),
          
          // Cart items summary
          if (widget.cartState.isEmpty)
            Container(
              key: const InteractionKey(
                'empty-cart-warning',
                description: 'Warning that cart is empty',
              ),
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.orange[50],
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.orange[200]!),
              ),
              child: const Row(
                children: [
                  Icon(Icons.warning, color: Colors.orange),
                  SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Your cart is empty. Add items before checkout.',
                      style: TextStyle(color: Colors.orange),
                    ),
                  ),
                ],
              ),
            )
          else
            ...widget.cartState.items.map((item) {
              return ListTile(
                key: InteractionKey(
                  'review-item-${item.productId}',
                  description: 'Review: ${item.name}',
                ),
                title: Text(item.name),
                subtitle: Text('\$${item.price.toStringAsFixed(2)} x ${item.quantity}'),
                trailing: Text(
                  '\$${(item.price * item.quantity).toStringAsFixed(2)}',
                  style: const TextStyle(fontWeight: FontWeight.bold),
                ),
              );
            }),
          
          const Divider(height: 32),
          
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                'Total:',
                style: TextStyle(fontSize: 18),
              ),
              Text(
                '\$${widget.cartState.total.toStringAsFixed(2)}',
                key: const InteractionKey(
                  'review-total',
                  description: 'Total amount to pay',
                ),
                style: const TextStyle(
                  fontSize: 24,
                  fontWeight: FontWeight.bold,
                  color: Colors.deepPurple,
                ),
              ),
            ],
          ),
          
          const SizedBox(height: 24),
          
          FilledButton(
            key: const InteractionKey(
              'continue-to-shipping-btn',
              description: 'Continue to shipping step',
            ),
            onPressed: () => widget.checkoutState.nextStep(),
            child: const Text('Continue to Shipping'),
          ),
        ],
      ),
    );
  }
  
  Widget _buildShippingForm(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Shipping Address',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 16),
            
            TextFormField(
              key: const InteractionKey(
                'shipping-name-input',
                description: 'Full name input',
              ),
              controller: _nameController,
              decoration: const InputDecoration(
                labelText: 'Full Name',
                border: OutlineInputBorder(),
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter your name';
                }
                return null;
              },
            ),
            const SizedBox(height: 12),
            
            TextFormField(
              key: const InteractionKey(
                'shipping-address-input',
                description: 'Street address input',
              ),
              controller: _addressController,
              decoration: const InputDecoration(
                labelText: 'Street Address',
                border: OutlineInputBorder(),
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter your address';
                }
                return null;
              },
            ),
            const SizedBox(height: 12),
            
            Row(
              children: [
                Expanded(
                  flex: 2,
                  child: TextFormField(
                    key: const InteractionKey(
                      'shipping-city-input',
                      description: 'City input',
                    ),
                    controller: _cityController,
                    decoration: const InputDecoration(
                      labelText: 'City',
                      border: OutlineInputBorder(),
                    ),
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return 'Required';
                      }
                      return null;
                    },
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextFormField(
                    key: const InteractionKey(
                      'shipping-zip-input',
                      description: 'ZIP code input',
                    ),
                    controller: _zipController,
                    decoration: const InputDecoration(
                      labelText: 'ZIP',
                      border: OutlineInputBorder(),
                    ),
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return 'Required';
                      }
                      return null;
                    },
                  ),
                ),
              ],
            ),
            
            const SizedBox(height: 24),
            
            Row(
              children: [
                OutlinedButton(
                  key: const InteractionKey(
                    'back-to-cart-btn',
                    description: 'Go back to cart review',
                  ),
                  onPressed: () => widget.checkoutState.previousStep(),
                  child: const Text('Back'),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton(
                    key: const InteractionKey(
                      'continue-to-payment-btn',
                      description: 'Continue to payment step',
                    ),
                    onPressed: () {
                      if (_formKey.currentState!.validate()) {
                        widget.checkoutState.nextStep();
                      }
                    },
                    child: const Text('Continue to Payment'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
  
  Widget _buildPaymentForm(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Payment Details',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 16),
          
          if (!widget.authState.isAuthenticated)
            Container(
              key: const InteractionKey(
                'auth-warning',
                description: 'Warning that user is not authenticated',
              ),
              padding: const EdgeInsets.all(16),
              margin: const EdgeInsets.only(bottom: 16),
              decoration: BoxDecoration(
                color: Colors.red[50],
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red[200]!),
              ),
              child: const Row(
                children: [
                  Icon(Icons.error, color: Colors.red),
                  SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'You must be logged in to complete checkout.',
                      style: TextStyle(color: Colors.red),
                    ),
                  ),
                ],
              ),
            ),
          
          TextFormField(
            key: const InteractionKey(
              'card-number-input',
              description: 'Credit card number input',
            ),
            controller: _cardNumberController,
            decoration: const InputDecoration(
              labelText: 'Card Number',
              border: OutlineInputBorder(),
              prefixIcon: Icon(Icons.credit_card),
            ),
            keyboardType: TextInputType.number,
          ),
          const SizedBox(height: 12),
          
          Row(
            children: [
              Expanded(
                child: TextFormField(
                  key: const InteractionKey(
                    'card-expiry-input',
                    description: 'Card expiry date input',
                  ),
                  controller: _expiryController,
                  decoration: const InputDecoration(
                    labelText: 'MM/YY',
                    border: OutlineInputBorder(),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: TextFormField(
                  key: const InteractionKey(
                    'card-cvv-input',
                    description: 'Card CVV input',
                  ),
                  controller: _cvvController,
                  decoration: const InputDecoration(
                    labelText: 'CVV',
                    border: OutlineInputBorder(),
                  ),
                  obscureText: true,
                  keyboardType: TextInputType.number,
                ),
              ),
            ],
          ),
          
          const SizedBox(height: 24),
          
          // Error display
          if (widget.checkoutState.error != null)
            Container(
              key: const InteractionKey(
                'payment-error',
                description: 'Payment error message',
              ),
              padding: const EdgeInsets.all(12),
              margin: const EdgeInsets.only(bottom: 16),
              decoration: BoxDecoration(
                color: Colors.red[50],
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red[200]!),
              ),
              child: Row(
                children: [
                  const Icon(Icons.error_outline, color: Colors.red),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      widget.checkoutState.error!,
                      style: const TextStyle(color: Colors.red),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.red),
                    onPressed: () => widget.checkoutState.clearError(),
                  ),
                ],
              ),
            ),
          
          // Order summary
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Colors.grey[100],
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('Total to pay:'),
                Text(
                  '\$${widget.cartState.total.toStringAsFixed(2)}',
                  key: const InteractionKey(
                    'payment-total',
                    description: 'Total payment amount',
                  ),
                  style: const TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
          
          const SizedBox(height: 24),
          
          Row(
            children: [
              OutlinedButton(
                key: const InteractionKey(
                  'back-to-shipping-btn',
                  description: 'Go back to shipping',
                ),
                onPressed: widget.checkoutState.isProcessing 
                  ? null 
                  : () => widget.checkoutState.previousStep(),
                child: const Text('Back'),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: FilledButton(
                  key: const InteractionKey(
                    'place-order-btn',
                    description: 'Submit order and process payment',
                  ),
                  onPressed: widget.checkoutState.isProcessing
                    ? null
                    : _processPayment,
                  child: widget.checkoutState.isProcessing
                    ? const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: Colors.white,
                        ),
                      )
                    : const Text('Place Order'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
  
  Widget _buildConfirmation(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(
              Icons.check_circle,
              size: 80,
              color: Colors.green,
            ),
            const SizedBox(height: 24),
            Text(
              'Order Confirmed!',
              key: const InteractionKey(
                'order-confirmed-title',
                description: 'Order confirmation title',
              ),
              style: Theme.of(context).textTheme.headlineMedium,
            ),
            const SizedBox(height: 8),
            Text(
              'Order ID: ${widget.checkoutState.orderId}',
              key: const InteractionKey(
                'order-id-display',
                description: 'Order ID',
              ),
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: Colors.grey[600],
              ),
            ),
            const SizedBox(height: 32),
            FilledButton(
              key: const InteractionKey(
                'continue-after-order-btn',
                description: 'Continue shopping after order',
              ),
              onPressed: () {
                widget.checkoutState.reset();
                widget.onOrderComplete();
              },
              child: const Text('Continue Shopping'),
            ),
          ],
        ),
      ),
    );
  }
  
  Future<void> _processPayment() async {
    final success = await widget.checkoutState.processCheckout({
      'cardNumber': _cardNumberController.text,
      'expiry': _expiryController.text,
      'cvv': _cvvController.text,
      'method': 'credit_card',
    });
    
    if (success) {
      // Order completed - cart is automatically cleared by API
    }
  }
}
