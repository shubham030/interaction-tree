import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import 'interaction_action.dart';
import 'interaction_capability.dart';
import 'interaction_key.dart';

class InteractionTarget {
  InteractionTarget({
    required this.key,
    required this.element,
    required this.capabilities,
    this.actions = const [],
  });

  final InteractionKey key;
  final Element element;
  final Set<InteractionCapability> capabilities;
  final List<InteractionAction> actions;

  String get id => key.id;
  String? get description => key.description;
  String? get semanticLabel => key.semanticLabel;

  Widget get widget => element.widget;
  String get widgetType => widget.runtimeType.toString();

  RenderBox? get renderBox {
    final renderObject = element.renderObject;
    if (renderObject is RenderBox && renderObject.hasSize) {
      return renderObject;
    }
    return null;
  }

  Offset get center {
    final box = renderBox;
    if (box == null) return Offset.zero;
    return box.localToGlobal(box.size.center(Offset.zero));
  }

  Rect get bounds {
    final box = renderBox;
    if (box == null) return Rect.zero;
    final topLeft = box.localToGlobal(Offset.zero);
    return topLeft & box.size;
  }

  bool get isVisible {
    final box = renderBox;
    if (box == null) return false;
    if (!box.hasSize || box.size.width <= 0 || box.size.height <= 0) {
      return false;
    }

    // Check if any ancestor is offstage (e.g., inactive Navigator routes)
    RenderObject? current = box;
    while (current != null) {
      // Check for Offstage widgets (used by Navigator for inactive routes)
      if (current is RenderOffstage && current.offstage) {
        return false;
      }
      // Check if not painting (e.g., Opacity(0))
      if (current is RenderOpacity && current.opacity == 0) {
        return false;
      }
      current = current.parent;
    }

    return true;
  }

  /// Extract the current text value from a text field, if available.
  String? get textValue {
    final renderObject = _findRenderEditable(element);
    if (renderObject != null) {
      return renderObject.plainText;
    }
    return null;
  }

  /// Check if this target is enabled (not disabled).
  /// Returns true if we can't determine the state.
  bool get isEnabled {
    // Check widget-level enabled property
    final w = widget;
    if (w is IconButton) return w.onPressed != null;
    if (w is TextButton) return w.onPressed != null;
    if (w is ElevatedButton) return w.onPressed != null;
    if (w is OutlinedButton) return w.onPressed != null;
    if (w is FilledButton) return w.onPressed != null;
    if (w is FloatingActionButton) return w.onPressed != null;
    return true; // Assume enabled if we can't determine
  }

  /// Get state information for this target.
  /// Returns a map with available state properties.
  Map<String, Object?> getState() {
    final state = <String, Object?>{};

    // Text value for text fields
    final text = textValue;
    if (text != null) {
      state['text'] = text;
    }

    // Enabled state
    state['enabled'] = isEnabled;

    // Visibility
    state['visible'] = isVisible;

    return state;
  }

  RenderEditable? _findRenderEditable(Element element) {
    RenderEditable? result;
    element.visitChildren((child) {
      if (result != null) return;
      final renderObject = child.renderObject;
      if (renderObject is RenderEditable) {
        result = renderObject;
      } else {
        result = _findRenderEditable(child);
      }
    });
    return result;
  }

  InteractionAction? findAction(String name) {
    for (final action in actions) {
      if (action.name == name) return action;
    }
    return null;
  }

  Map<String, Object?> toJson({
    bool includeBounds = false,
    bool includeWidgetType = false,
  }) =>
      {
        'id': id,
        if (description != null) 'description': description,
        if (semanticLabel != null) 'semanticLabel': semanticLabel,
        'capabilities': capabilities.map((c) => c.name).toList(),
        if (actions.isNotEmpty)
          'actions': actions.map((a) => a.toJson()).toList(),
        if (includeWidgetType) 'widgetType': widgetType,
        if (includeBounds)
          'bounds': {
            'x': bounds.left,
            'y': bounds.top,
            'width': bounds.width,
            'height': bounds.height,
          },
      };

  @override
  String toString() => 'InteractionTarget($id)';
}
