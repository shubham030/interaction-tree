/// Simulated API service for the example app.

import 'dart:async';
import 'dart:math';

/// Simulates network delay
Future<void> _simulateNetworkDelay() async {
  await Future.delayed(Duration(milliseconds: 200 + Random().nextInt(300)));
}

/// API Exception types for debugging
class ApiException implements Exception {
  final String message;
  final int? statusCode;
  final String? endpoint;
  
  ApiException(this.message, {this.statusCode, this.endpoint});
  
  @override
  String toString() => 'ApiException: $message (status: $statusCode, endpoint: $endpoint)';
}

class NetworkTimeoutException extends ApiException {
  NetworkTimeoutException(String endpoint) 
    : super('Connection timed out', statusCode: 408, endpoint: endpoint);
}

class UnauthorizedException extends ApiException {
  UnauthorizedException() 
    : super('Unauthorized - invalid or expired token', statusCode: 401);
}

class ServerException extends ApiException {
  ServerException(String message) 
    : super(message, statusCode: 500);
}

/// Simulated API Service
class ApiService {
  static final ApiService instance = ApiService._();
  ApiService._();
  
  // Configuration for testing different failure modes
  bool simulateTimeout = false;
  bool simulateAuthFailure = false;
  bool simulateServerError = false;
  int timeoutMs = 5000;
  
  // Simulated user session
  String? _authToken;
  bool get isAuthenticated => _authToken != null;
  
  /// Login endpoint
  Future<Map<String, dynamic>> login(String email, String password) async {
    await _simulateNetworkDelay();
    
    if (simulateTimeout) {
      await Future.delayed(Duration(milliseconds: timeoutMs));
      throw NetworkTimeoutException('/auth/login');
    }
    
    // Simulate auth failure
    if (simulateAuthFailure) {
      throw UnauthorizedException();
    }
    
    // Simulate server error
    if (simulateServerError) {
      throw ServerException('Internal server error during login');
    }
    
    // Validate credentials (simple simulation)
    if (email.isEmpty || password.isEmpty) {
      throw ApiException('Email and password required', statusCode: 400);
    }
    
    if (password.length < 6) {
      throw ApiException('Invalid credentials', statusCode: 401);
    }
    
    // Success!
    _authToken = 'fake-token-${DateTime.now().millisecondsSinceEpoch}';
    
    return {
      'success': true,
      'token': _authToken,
      'user': {
        'email': email,
        'name': email.split('@').first,
      },
    };
  }
  
  /// Logout endpoint
  Future<void> logout() async {
    await _simulateNetworkDelay();
    _authToken = null;
  }
  
  /// Get cart items
  Future<List<Map<String, dynamic>>> getCartItems() async {
    await _simulateNetworkDelay();
    
    // This returns a snapshot that might be stale
    return List.from(_cartItems);
  }
  
  // Simulated cart storage
  final List<Map<String, dynamic>> _cartItems = [];
  int _cartVersion = 0;
  
  /// Add item to cart
  Future<Map<String, dynamic>> addToCart(String productId, int quantity) async {
    // Simulate a slow operation that can cause race conditions
    final versionBefore = _cartVersion;
    await _simulateNetworkDelay();
    
    final existingIndex = _cartItems.indexWhere((item) => item['productId'] == productId);
    
    if (existingIndex >= 0) {
      _cartItems[existingIndex]['quantity'] += quantity;
    } else {
      _cartItems.add({
        'productId': productId,
        'name': 'Product $productId',
        'price': (10 + Random().nextInt(90)).toDouble(),
        'quantity': quantity,
      });
    }
    
    _cartVersion++;
    
    return {
      'success': true,
      'cartVersion': _cartVersion,
      'itemCount': _cartItems.length,
      'total': _calculateTotal(),
    };
  }
  
  /// Remove item from cart
  Future<void> removeFromCart(String productId) async {
    await _simulateNetworkDelay();
    _cartItems.removeWhere((item) => item['productId'] == productId);
    _cartVersion++;
  }
  
  /// Clear cart
  Future<void> clearCart() async {
    await _simulateNetworkDelay();
    _cartItems.clear();
    _cartVersion++;
  }
  
  double _calculateTotal() {
    return _cartItems.fold(0.0, (sum, item) {
      return sum + (item['price'] as double) * (item['quantity'] as int);
    });
  }
  
  /// Process checkout
  Future<Map<String, dynamic>> processCheckout(Map<String, dynamic> paymentInfo) async {
    await _simulateNetworkDelay();
    
    if (simulateServerError) {
      throw ServerException('Payment processing failed');
    }
    
    if (simulateTimeout) {
      await Future.delayed(Duration(milliseconds: timeoutMs));
      throw NetworkTimeoutException('/checkout');
    }
    
    // Validate payment info
    if (paymentInfo['cardNumber'] == null || paymentInfo['cardNumber'].toString().length < 16) {
      throw ApiException('Invalid card number', statusCode: 400);
    }
    
    // Process payment (simulated)
    final orderId = 'ORD-${DateTime.now().millisecondsSinceEpoch}';
    final total = _calculateTotal();
    
    // Clear cart after successful checkout
    _cartItems.clear();
    _cartVersion++;
    
    return {
      'success': true,
      'orderId': orderId,
      'total': total,
      'status': 'confirmed',
    };
  }
  
  /// Get user preferences
  Future<Map<String, dynamic>> getUserPreferences() async {
    await _simulateNetworkDelay();
    
    if (simulateTimeout) {
      throw NetworkTimeoutException('/user/preferences');
    }
    
    if (!isAuthenticated) {
      throw UnauthorizedException();
    }
    
    return {
      'theme': 'light',
      'notifications': true,
      'language': 'en',
    };
  }
  
  /// Reset all failure simulations
  void resetFailureSimulation() {
    simulateTimeout = false;
    simulateAuthFailure = false;
    simulateServerError = false;
  }
}
