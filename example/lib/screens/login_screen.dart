/// Login screen for Debug Agent testing.

import 'package:flutter/material.dart';
import 'package:interaction_tree/interaction_tree.dart';
import '../state/app_state.dart';
import '../services/api_service.dart';

class LoginScreen extends StatefulWidget {
  final AuthState authState;
  final VoidCallback onLoginSuccess;
  
  const LoginScreen({
    super.key,
    required this.authState,
    required this.onLoginSuccess,
  });

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  
  bool _obscurePassword = true;
  
  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Login'),
        actions: [
          // Debug controls for testing different failure modes
          PopupMenuButton<String>(
            key: const InteractionKey(
              'debug-menu-btn',
              description: 'Open debug menu to configure failure modes',
            ),
            icon: const Icon(Icons.bug_report),
            onSelected: (value) {
              final api = ApiService.instance;
              setState(() {
                switch (value) {
                  case 'timeout':
                    api.simulateTimeout = !api.simulateTimeout;
                    _showDebugSnackbar('Timeout: ${api.simulateTimeout}');
                  case 'auth_fail':
                    api.simulateAuthFailure = !api.simulateAuthFailure;
                    _showDebugSnackbar('Auth Failure: ${api.simulateAuthFailure}');
                  case 'server_error':
                    api.simulateServerError = !api.simulateServerError;
                    _showDebugSnackbar('Server Error: ${api.simulateServerError}');
                  case 'reset':
                    api.resetFailureSimulation();
                    _showDebugSnackbar('All failures reset');
                }
              });
            },
            itemBuilder: (context) => [
              PopupMenuItem(
                key: const InteractionKey(
                  'toggle-timeout-opt',
                  description: 'Toggle network timeout simulation',
                ),
                value: 'timeout',
                child: Row(
                  children: [
                    Icon(ApiService.instance.simulateTimeout 
                      ? Icons.check_box 
                      : Icons.check_box_outline_blank),
                    const SizedBox(width: 8),
                    const Text('Simulate Timeout'),
                  ],
                ),
              ),
              PopupMenuItem(
                key: const InteractionKey(
                  'toggle-auth-fail-opt',
                  description: 'Toggle auth failure simulation',
                ),
                value: 'auth_fail',
                child: Row(
                  children: [
                    Icon(ApiService.instance.simulateAuthFailure 
                      ? Icons.check_box 
                      : Icons.check_box_outline_blank),
                    const SizedBox(width: 8),
                    const Text('Simulate Auth Failure'),
                  ],
                ),
              ),
              PopupMenuItem(
                key: const InteractionKey(
                  'toggle-server-error-opt',
                  description: 'Toggle server error simulation',
                ),
                value: 'server_error',
                child: Row(
                  children: [
                    Icon(ApiService.instance.simulateServerError 
                      ? Icons.check_box 
                      : Icons.check_box_outline_blank),
                    const SizedBox(width: 8),
                    const Text('Simulate Server Error'),
                  ],
                ),
              ),
              const PopupMenuDivider(),
              const PopupMenuItem(
                value: 'reset',
                child: Row(
                  children: [
                    Icon(Icons.refresh),
                    SizedBox(width: 8),
                    Text('Reset All'),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
      body: ListenableBuilder(
        listenable: widget.authState,
        builder: (context, _) {
          return SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: Form(
              key: _formKey,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Icon(
                    Icons.lock_outline,
                    size: 80,
                    color: Colors.deepPurple,
                  ),
                  const SizedBox(height: 32),
                  
                  Text(
                    'Welcome Back',
                    style: Theme.of(context).textTheme.headlineMedium,
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Sign in to continue',
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Colors.grey[600],
                    ),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 32),
                  
                  // Email field
                  TextFormField(
                    key: const InteractionKey(
                      'email-input',
                      description: 'Email address input field',
                    ),
                    controller: _emailController,
                    keyboardType: TextInputType.emailAddress,
                    decoration: const InputDecoration(
                      labelText: 'Email',
                      prefixIcon: Icon(Icons.email_outlined),
                      border: OutlineInputBorder(),
                    ),
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return 'Please enter your email';
                      }
                      return null;
                    },
                  ),
                  const SizedBox(height: 16),
                  
                  // Password field
                  TextFormField(
                    key: const InteractionKey(
                      'password-input',
                      description: 'Password input field',
                    ),
                    controller: _passwordController,
                    obscureText: _obscurePassword,
                    decoration: InputDecoration(
                      labelText: 'Password',
                      prefixIcon: const Icon(Icons.lock_outlined),
                      border: const OutlineInputBorder(),
                      suffixIcon: IconButton(
                        key: const InteractionKey(
                          'toggle-password-visibility',
                          description: 'Toggle password visibility',
                        ),
                        icon: Icon(
                          _obscurePassword 
                            ? Icons.visibility_outlined 
                            : Icons.visibility_off_outlined,
                        ),
                        onPressed: () {
                          setState(() => _obscurePassword = !_obscurePassword);
                        },
                      ),
                    ),
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return 'Please enter your password';
                      }
                      return null;
                    },
                  ),
                  const SizedBox(height: 24),
                  
                  // Error message
                  if (widget.authState.error != null)
                    Container(
                      key: const InteractionKey(
                        'error-message',
                        description: 'Login error message display',
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
                              widget.authState.error!,
                              style: const TextStyle(color: Colors.red),
                            ),
                          ),
                          IconButton(
                            key: const InteractionKey(
                              'dismiss-error-btn',
                              description: 'Dismiss error message',
                            ),
                            icon: const Icon(Icons.close, color: Colors.red),
                            onPressed: () => widget.authState.clearError(),
                          ),
                        ],
                      ),
                    ),
                  
                  // Login button
                  FilledButton(
                    key: const InteractionKey(
                      'login-btn',
                      description: 'Submit login form',
                    ),
                    onPressed: _handleLogin,
                    child: widget.authState.isLoading
                      ? const SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Text('Sign In'),
                  ),
                  
                  const SizedBox(height: 16),
                  
                  // Quick login for testing
                  OutlinedButton(
                    key: const InteractionKey(
                      'quick-login-btn',
                      description: 'Quick login with test credentials',
                    ),
                    onPressed: widget.authState.isLoading ? null : _handleQuickLogin,
                    child: const Text('Quick Login (test@example.com)'),
                  ),
                  
                  const SizedBox(height: 32),
                  
                  // Debug info
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.grey[100],
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Debug Info',
                          style: Theme.of(context).textTheme.labelSmall,
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'isLoading: ${widget.authState.isLoading}\n'
                          'isAuthenticated: ${widget.authState.isAuthenticated}\n'
                          'timeout: ${ApiService.instance.simulateTimeout}\n'
                          'authFail: ${ApiService.instance.simulateAuthFailure}\n'
                          'serverError: ${ApiService.instance.simulateServerError}',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            fontFamily: 'monospace',
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
  
  void _handleLogin() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    
    final success = await widget.authState.login(
      _emailController.text,
      _passwordController.text,
    );
    
    if (success && mounted) {
      widget.onLoginSuccess();
    }
  }
  
  Future<void> _handleQuickLogin() async {
    _emailController.text = 'test@example.com';
    _passwordController.text = 'password123';
    _handleLogin();
  }
  
  void _showDebugSnackbar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        duration: const Duration(seconds: 1),
      ),
    );
  }
}
