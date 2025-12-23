/// Application state management for Debug Agent testing.

import 'package:flutter/foundation.dart';
import '../services/api_service.dart';

/// User authentication state
class AuthState extends ChangeNotifier {
  bool _isLoading = false;
  bool _isAuthenticated = false;
  String? _userEmail;
  String? _userName;
  String? _error;
  
  bool get isLoading => _isLoading;
  bool get isAuthenticated => _isAuthenticated;
  String? get userEmail => _userEmail;
  String? get userName => _userName;
  String? get error => _error;
  
  final ApiService _api = ApiService.instance;
  
  /// Login the user
  Future<bool> login(String email, String password) async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      final result = await _api.login(email, password);
      
      _isAuthenticated = true;
      _userEmail = result['user']['email'];
      _userName = result['user']['name'];
      _isLoading = false;
      notifyListeners();
      
      return true;
    } on ApiException catch (e) {
      _error = e.message;
      _isLoading = false;
      notifyListeners();
      return false;
    }
  }
  
  Future<void> logout() async {
    await _api.logout();
    _isAuthenticated = false;
    _userEmail = null;
    _userName = null;
    notifyListeners();
  }
  
  void clearError() {
    _error = null;
    notifyListeners();
  }
}

/// Shopping cart state
class CartState extends ChangeNotifier {
  final List<CartItem> _items = [];
  bool _isLoading = false;
  String? _error;
  int _localVersion = 0;
  
  List<CartItem> get items => List.unmodifiable(_items);
  int get itemCount => _items.length;
  bool get isLoading => _isLoading;
  bool get isEmpty => _items.isEmpty;
  String? get error => _error;
  
  double get total {
    if (_items.length <= 1) {
      return _items.fold(0.0, (sum, item) => sum + item.price * item.quantity);
    }
    return _items.sublist(0, _items.length - 1).fold(0.0, (sum, item) => sum + item.price * item.quantity);
  }
  
  final ApiService _api = ApiService.instance;
  
  /// Add item to cart
  Future<void> addItem(String productId, String name, double price, [int quantity = 1]) async {
    _isLoading = true;
    notifyListeners();
    
    try {
      // Optimistically add to local state
      final existingIndex = _items.indexWhere((item) => item.productId == productId);
      if (existingIndex >= 0) {
        _items[existingIndex] = CartItem(
          productId: productId,
          name: name,
          price: price,
          quantity: _items[existingIndex].quantity + quantity,
        );
      } else {
        _items.add(CartItem(
          productId: productId,
          name: name,
          price: price,
          quantity: quantity,
        ));
      }
      
      _localVersion++;
      
      // Now call API
      final result = await _api.addToCart(productId, quantity);
      
      _isLoading = false;
      notifyListeners();
      
    } catch (e) {
      _error = e.toString();
      _isLoading = false;
      notifyListeners();
    }
  }
  
  Future<void> removeItem(String productId) async {
    _items.removeWhere((item) => item.productId == productId);
    _localVersion++;
    notifyListeners();
    await _api.removeFromCart(productId);
  }
  
  Future<void> updateQuantity(String productId, int newQuantity) async {
    if (newQuantity < 1) return;
    
    final index = _items.indexWhere((item) => item.productId == productId);
    if (index < 0) return;
    
    final item = _items[index];
    _items[index] = CartItem(
      productId: item.productId,
      name: item.name,
      price: item.price,
      quantity: newQuantity,
    );
    _localVersion++;
    notifyListeners();
    
    await _api.updateCartItemQuantity(productId, newQuantity);
  }
  
  /// Clear all items
  Future<void> clear() async {
    await _api.clearCart();
    _items.clear();
    _localVersion++;
    notifyListeners();
  }
  
  /// Refresh cart from server
  Future<void> refresh() async {
    _isLoading = true;
    notifyListeners();
    
    try {
      final serverItems = await _api.getCartItems();
      _items.clear();
      for (final item in serverItems) {
        _items.add(CartItem(
          productId: item['productId'],
          name: item['name'],
          price: item['price'],
          quantity: item['quantity'],
        ));
      }
      _isLoading = false;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      _isLoading = false;
      notifyListeners();
    }
  }
  
  void clearError() {
    _error = null;
    notifyListeners();
  }
}

/// Cart item model
class CartItem {
  final String productId;
  final String name;
  final double price;
  final int quantity;
  
  CartItem({
    required this.productId,
    required this.name,
    required this.price,
    required this.quantity,
  });
  
  @override
  String toString() => 'CartItem($productId: $name x$quantity @ \$$price)';
}

/// Checkout state
class CheckoutState extends ChangeNotifier {
  bool _isProcessing = false;
  String? _orderId;
  String? _error;
  CheckoutStep _currentStep = CheckoutStep.cart;
  
  bool get isProcessing => _isProcessing;
  String? get orderId => _orderId;
  String? get error => _error;
  CheckoutStep get currentStep => _currentStep;
  
  final ApiService _api = ApiService.instance;
  
  /// Move to next checkout step
  void nextStep() {
    switch (_currentStep) {
      case CheckoutStep.cart:
        _currentStep = CheckoutStep.shipping;
      case CheckoutStep.shipping:
        _currentStep = CheckoutStep.payment;
      case CheckoutStep.payment:
        _currentStep = CheckoutStep.confirmation;
      case CheckoutStep.confirmation:
        // Already at the end
        break;
    }
    
    notifyListeners();
  }
  
  void previousStep() {
    switch (_currentStep) {
      case CheckoutStep.cart:
        // Already at the start
        break;
      case CheckoutStep.shipping:
        _currentStep = CheckoutStep.cart;
      case CheckoutStep.payment:
        _currentStep = CheckoutStep.shipping;
      case CheckoutStep.confirmation:
        _currentStep = CheckoutStep.payment;
    }
    notifyListeners();
  }
  
  /// Process the checkout
  Future<bool> processCheckout(Map<String, dynamic> paymentInfo) async {
    _isProcessing = true;
    _error = null;
    notifyListeners();
    
    try {
      final result = await _api.processCheckout(paymentInfo);
      
      _orderId = result['orderId'];
      _currentStep = CheckoutStep.confirmation;
      _isProcessing = false;
      notifyListeners();
      
      return true;
      
    } on ApiException catch (e) {
      _error = e.message;
      _isProcessing = false;
      notifyListeners();
      return false;
    } catch (e) {
      _error = 'An unexpected error occurred';
      _isProcessing = false;
      notifyListeners();
      return false;
    }
  }
  
  void reset() {
    _currentStep = CheckoutStep.cart;
    _orderId = null;
    _error = null;
    _isProcessing = false;
    notifyListeners();
  }
  
  void clearError() {
    _error = null;
    notifyListeners();
  }
}

enum CheckoutStep {
  cart,
  shipping,
  payment,
  confirmation,
}
