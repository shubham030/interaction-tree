import 'dart:convert';
import 'dart:developer' as developer;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../core/interactable_mixin.dart';
import '../core/interaction_action.dart';
import '../core/interaction_capability.dart';
import '../core/interaction_context.dart';
import '../core/interaction_key.dart';
import '../core/interaction_target.dart';
import 'interaction_executor.dart';

/// Service that registers VM service extensions for the interaction tree.
///
/// Call [InteractionTreeService.ensureInitialized] in your app's main() to enable
/// external tools (like MCP servers or LLMs) to interact with your app.
///
/// ```dart
/// void main() {
///   InteractionTreeService.ensureInitialized();
///   runApp(MyApp());
/// }
/// ```
class InteractionTreeService {
  InteractionTreeService._();

  static InteractionTreeService? _instance;
  static InteractionTreeService get instance {
    _instance ??= InteractionTreeService._();
    return _instance!;
  }

  bool _initialized = false;

  /// Initialize the service and register VM service extensions.
  /// Safe to call multiple times - only initializes once.
  static void ensureInitialized() {
    instance._initialize();
  }

  void _initialize() {
    if (_initialized) return;
    _initialized = true;

    // Only register extensions in debug/profile mode
    if (kReleaseMode) return;

    _registerExtensions();
  }

  void _registerExtensions() {
    // ext.interaction_tree.getTree
    developer.registerExtension(
      'ext.interaction_tree.getTree',
      (method, params) async {
        try {
          final includeBounds = params['includeBounds'] == 'true';
          final includeWidgetType = params['includeWidgetType'] == 'true';
          final includeState = params['includeState'] == 'true';

          final targets = _collectTargets(
            includeBounds: includeBounds,
            includeWidgetType: includeWidgetType,
            includeState: includeState,
          );

          return developer.ServiceExtensionResponse.result(
            jsonEncode({'targets': targets}),
          );
        } catch (e, st) {
          return developer.ServiceExtensionResponse.error(
            developer.ServiceExtensionResponse.extensionError,
            'Failed to get tree: $e\n$st',
          );
        }
      },
    );

    // ext.interaction_tree.execute
    developer.registerExtension(
      'ext.interaction_tree.execute',
      (method, params) async {
        try {
          final id = params['id'];
          final interaction = params['interaction'];
          final argsJson = params['args'];
          final returnTree = params['returnTree'] != 'false'; // default true

          if (id == null || interaction == null) {
            return developer.ServiceExtensionResponse.error(
              developer.ServiceExtensionResponse.invalidParams,
              'Missing required params: id and interaction',
            );
          }

          Map<String, dynamic>? args;
          if (argsJson != null && argsJson.isNotEmpty) {
            args = jsonDecode(argsJson) as Map<String, dynamic>;
          }

          final result = await InteractionExecutor.execute(
            id: id,
            interaction: interaction,
            args: args,
          );

          // Include updated tree by default after interaction settles
          if (returnTree && result['success'] == true) {
            result['tree'] = InteractionExecutor.getSettledTree(
              includeBounds: true,
              includeWidgetType: true,
            );
          }

          return developer.ServiceExtensionResponse.result(jsonEncode(result));
        } catch (e, st) {
          return developer.ServiceExtensionResponse.error(
            developer.ServiceExtensionResponse.extensionError,
            'Failed to execute: $e\n$st',
          );
        }
      },
    );

    // ext.interaction_tree.getState
    developer.registerExtension(
      'ext.interaction_tree.getState',
      (method, params) async {
        try {
          final id = params['id'];
          if (id == null) {
            return developer.ServiceExtensionResponse.error(
              developer.ServiceExtensionResponse.invalidParams,
              'Missing required param: id',
            );
          }

          final target = _findTarget(id);
          if (target == null) {
            return developer.ServiceExtensionResponse.result(
              jsonEncode({'error': 'Target not found: $id'}),
            );
          }

          final state = target.getState();
          return developer.ServiceExtensionResponse.result(jsonEncode(state));
        } catch (e, st) {
          return developer.ServiceExtensionResponse.error(
            developer.ServiceExtensionResponse.extensionError,
            'Failed to get state: $e\n$st',
          );
        }
      },
    );

    // ext.interaction_tree.batch
    developer.registerExtension(
      'ext.interaction_tree.batch',
      (method, params) async {
        try {
          final stepsJson = params['steps'];
          final returnTree = params['returnTree'] != 'false'; // default true

          if (stepsJson == null) {
            return developer.ServiceExtensionResponse.error(
              developer.ServiceExtensionResponse.invalidParams,
              'Missing required param: steps',
            );
          }

          final steps = jsonDecode(stepsJson) as List<dynamic>;
          final results = <Map<String, dynamic>>[];
          var success = true;
          int? stoppedAtIndex;

          for (var i = 0; i < steps.length; i++) {
            final step = steps[i] as Map<String, dynamic>;
            final action = step['action'] as String;
            final id = step['id'] as String;

            final result = await InteractionExecutor.execute(
              id: id,
              interaction: action,
              args: step,
            );

            results.add(result);

            if (result['success'] != true) {
              success = false;
              stoppedAtIndex = i;
              break;
            }
          }

          final response = <String, dynamic>{
            'success': success,
            'results': results,
            if (stoppedAtIndex != null) 'stoppedAtIndex': stoppedAtIndex,
          };

          // Include final tree state after batch completes
          if (returnTree) {
            response['tree'] = InteractionExecutor.getSettledTree(
              includeBounds: true,
              includeWidgetType: true,
            );
          }

          return developer.ServiceExtensionResponse.result(jsonEncode(response));
        } catch (e, st) {
          return developer.ServiceExtensionResponse.error(
            developer.ServiceExtensionResponse.extensionError,
            'Failed to execute batch: $e\n$st',
          );
        }
      },
    );
  }

  /// Collect all interaction targets from the widget tree.
  List<Map<String, dynamic>> _collectTargets({
    bool includeBounds = false,
    bool includeWidgetType = false,
    bool includeState = false,
  }) {
    final targets = <Map<String, dynamic>>[];
    final binding = WidgetsBinding.instance;

    void visit(Element element) {
      final key = element.widget.key;
      if (key is InteractionKey) {
        final target = InteractionTarget(
          key: key,
          element: element,
          capabilities: key.capabilities ?? inferCapabilities(element),
          actions: _getActionsForElement(element),
        );

        // Only include visible targets (filters out offstage/hidden widgets)
        if (!target.isVisible) {
          element.visitChildren(visit);
          return;
        }

        final json = target.toJson(
          includeBounds: includeBounds,
          includeWidgetType: includeWidgetType,
        );

        if (includeState) {
          json['state'] = target.getState();
        }

        // Collect ancestor InteractionContext info
        final contexts = InteractionContext.allOf(element);
        if (contexts.isNotEmpty) {
          json['contexts'] = contexts.map((c) => c.toJson()).toList();
        }

        targets.add(json);
      }

      element.visitChildren(visit);
    }

    binding.rootElement?.visitChildren(visit);
    return targets;
  }

  /// Find a target by ID.
  InteractionTarget? _findTarget(String id) {
    InteractionTarget? found;
    final binding = WidgetsBinding.instance;

    void visit(Element element) {
      if (found != null) return;

      final key = element.widget.key;
      if (key is InteractionKey && key.id == id) {
        found = InteractionTarget(
          key: key,
          element: element,
          capabilities: key.capabilities ?? inferCapabilities(element),
          actions: _getActionsForElement(element),
        );
        return;
      }

      element.visitChildren(visit);
    }

    binding.rootElement?.visitChildren(visit);
    return found;
  }

  /// Get actions from InteractableMixin if present.
  List<InteractionAction> _getActionsForElement(Element element) {
    if (element is StatefulElement) {
      final state = element.state;
      if (state is InteractableMixin) {
        return state.actions;
      }
    }
    return const [];
  }
}
