use std::{
    borrow::Cow,
    collections::HashMap,
    mem::{self, Discriminant, discriminant},
    str::FromStr,
    sync::LazyLock,
};

use crate::model::{
    Block, Buffer, Command, Device, Enum, EnumValue, EnumVariant, Extern, Field, FieldSet,
    Manifest, Object, Register,
};
use convert_case::Boundary;
use device_driver_common::{
    identifier::{All, Identifier, IdentifierRef, IdentifierType, Operation, Type},
    span::{Span, SpanExt, Spanned},
    specifiers::{
        Access, AddressMode, AddressRange, BaseType, ByteOrder, HwAccess, HwHandshake, HwKind,
        Integer, IntrTrigger, NodeType, OnRead, OnWrite, Precedence, Repeat, RepeatSource,
        ReservedBehavior, ResetValue, SvBus, TypeConversion,
    },
};
use device_driver_diagnostics::{
    Diagnostics,
    errors::{
        DuplicateProperty, ExternInvalidSizeBits, FieldAddressOutOfRange, FieldAddressWrongOrder,
        IgnoredDocCommentOnProperty, InvalidAutoIdentifier, InvalidExpressionType,
        InvalidIdentifier, InvalidNodeType, InvalidPropertyName, InvalidRepeat,
        InvalidShortProperty, InvalidSubnode, InvalidTypeConversion, InvalidTypeSpecifier,
        MissingRequiredProperty, ResetValueNegative, SizeBytesTooLarge, UnknownNodeType,
    },
};
use device_driver_parser::{Ast, Expression, Ident, Node, Property};
use itertools::Itertools;

pub fn lower(ast: Ast, diagnostics: &mut Diagnostics) -> Manifest {
    let Some(root_node) = ast.root_node else {
        return Default::default();
    };

    let result = lower_node(
        &root_node,
        None,
        None,
        &[NodeType::Manifest, NodeType::Device],
        diagnostics,
    );

    match result {
        LowerResult::Manifest(m) => m,
        LowerResult::Objects(Object::Device(d), siblings) => {
            assert!(siblings.is_empty(), "Device doesn't have sibling objects");
            d.into()
        }
        LowerResult::Objects(_, _) => unreachable!(),
        LowerResult::Error(_) => Default::default(),
    }
}

enum LowerResult {
    Manifest(Manifest),
    Objects(Object, Vec<Object>),
    Error(Vec<Object>),
}

fn lower_node(
    node: &Node,
    parent_node_type: Option<Spanned<NodeType>>,
    parent_node_name: Option<Ident>,
    allowed_node_types: &[NodeType],
    diagnostics: &mut Diagnostics,
) -> LowerResult {
    let Ok(node_type) = NodeType::from_str(node.node_type.val) else {
        diagnostics.add(UnknownNodeType {
            node_type: node.node_type.span,
            allowed_node_types: allowed_node_types.to_vec(),
        });
        return LowerResult::Error(Vec::new());
    };
    let node_type = node_type.with_span(node.node_type.span);

    if !allowed_node_types.contains(&node_type) {
        diagnostics.add(InvalidNodeType {
            node_type: node_type.span,
            parent_node_type,
            allowed_node_types: allowed_node_types.to_vec(),
        });
        return LowerResult::Error(Vec::new());
    }

    match node_type.value {
        NodeType::Manifest => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => {
                assert!(siblings.is_empty(), "Manifest has no siblings");
                LowerResult::Manifest(val)
            }
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Device => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Device(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Block => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Block(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Register => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Register(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Command => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Command(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Buffer => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Buffer(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::FieldSet => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::FieldSet(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Enum => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Enum(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Extern => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Extern(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
        NodeType::Field => match parse_node_to_shape(node, parent_node_name, diagnostics) {
            Ok((val, siblings)) => LowerResult::Objects(Object::Field(val), siblings),
            Err(siblings) => LowerResult::Error(siblings),
        },
    }
}

fn parse_node_to_shape<'src, S: Shape>(
    node: &Node<'src>,
    parent_node_name: Option<Ident<'src>>,
    diagnostics: &mut Diagnostics,
) -> Result<(S, Vec<Object>), Vec<Object>> {
    let mut target = S::default();
    let mut sibling_objects = Vec::new();
    let mut error = false;

    *target.span() = node.span;

    // Doc comments

    *target.doc_comments() = node.doc_comments.iter().map(|c| c.value).join("\n");

    // Object name

    match (node.name.is_auto(), parent_node_name) {
        (true, Some(parent_node_name)) => {
            match Identifier::try_parse(parent_node_name.val) {
                Ok(ident) => *target.name() = ident.with_span(node.name.span),
                Err(_e) => {
                    // We don't need to emit a diagnostic since the parent will already have a diagnostic.
                    // Can't continue with this node when there's no name
                    error = true;
                }
            }
        }
        (true, None) => {
            diagnostics.add(InvalidAutoIdentifier {
                auto_identifier: node.name.span,
            });
            // Can't continue with this node when there's no name
            error = true;
        }
        (false, _) => {
            match Identifier::try_parse(node.name.val) {
                Ok(ident) => *target.name() = ident.with_span(node.name.span),
                Err(e) => {
                    diagnostics.add(InvalidIdentifier::new(e, node.name.span));
                    // Can't continue with this node when there's no name
                    error = true;
                }
            }
        }
    }

    // Repeat

    match (target.repeat(), node.repeat) {
        (None, Some(node_repeat)) => {
            diagnostics.add(InvalidRepeat {
                repeat: node_repeat.span,
                node_type: S::NODE_TYPE.with_span(node.node_type.span),
            });
        }
        (Some(target_repeat), Some(node_repeat)) => {
            *target_repeat = Some(Repeat {
                source: match node_repeat.source {
                    device_driver_parser::RepeatSource::Count(count) => RepeatSource::Count(count),
                    device_driver_parser::RepeatSource::Enum(ident) => RepeatSource::Enum(
                        IdentifierRef::new(ident.val.into()).with_span(ident.span),
                    ),
                },
                stride: (node_repeat.stride.value as i128).with_span(node_repeat.stride.span),
            })
        }
        (_, None) => {}
    }

    // Base type

    match (target.base_type(), node.type_specifier.as_ref()) {
        (None, None) => {}
        (None, Some(type_specifier)) => {
            diagnostics.add(InvalidTypeSpecifier {
                node_type: S::NODE_TYPE.with_span(node.node_type.span),
                type_specifier: type_specifier.span,
            });
        }
        (Some(base_type), None) => *base_type = BaseType::Unspecified.with_dummy_span(),
        (Some(base_type), Some(type_specifier)) => {
            *base_type = type_specifier.base_type;
        }
    }

    // Conversion

    match (target.conversion_type(), node.type_specifier.as_ref()) {
        (None, Some(type_specifier)) if type_specifier.conversion.is_some() => {
            if target.base_type().is_some() {
                // Only emit this diagnostic if a base type is supported. Otherwise we'll get double diagnostics
                diagnostics.add(InvalidTypeConversion {
                    node_type: S::NODE_TYPE.with_span(node.node_type.span),
                    type_conversion: type_specifier.span.skip(type_specifier.base_type.span),
                });
            }
        }
        (None, _) => {}
        (Some(conversion_type), None) => *conversion_type = None,
        (Some(conversion_type), Some(type_specifier)) => {
            *conversion_type = type_specifier.conversion.as_ref().and_then(|c| {
                let reference = match c {
                    device_driver_parser::TypeConversion::Reference(ident) => {
                        Some(IdentifierRef::<Type>::new(ident.val.into()).with_span(ident.span))
                    }
                    device_driver_parser::TypeConversion::Subnode(sub_node) => {
                        let sub_node = lower_node(
                            sub_node,
                            Some(NodeType::Field.with_span(node.node_type.span)),
                            Some(node.name),
                            &[NodeType::Enum, NodeType::Extern],
                            diagnostics,
                        );

                        match sub_node {
                            LowerResult::Manifest(_) => unreachable!(),
                            LowerResult::Objects(object, objects) => {
                                let reference = object
                                    .name()
                                    .clone()
                                    // The only allowed subnodes are types, so this should be fine
                                    .cast_assert()
                                    .take_ref()
                                    .with_span(object.name_span());
                                sibling_objects.push(object);
                                sibling_objects.extend(objects);
                                Some(reference)
                            }
                            LowerResult::Error(objects) => {
                                sibling_objects.extend(objects);
                                None
                            }
                        }
                    }
                };

                reference.map(|reference| TypeConversion {
                    type_name: reference,
                    fallible: type_specifier.use_try,
                })
            })
        }
    }

    // Properties

    let mut possible_properties = S::supported_properties().to_vec();
    let mut removed_properties = HashMap::new();
    let mut removed_short_properties = HashMap::new();
    for property in &node.properties {
        let Some(property_info) = possible_properties
            .iter()
            .find(|p| p.name == PropertyName::Exact(property.name.val))
        else {
            if let Some(original) = removed_properties.get(property.name.val).copied() {
                diagnostics.add(DuplicateProperty {
                    original,
                    duplicate: property.name.span,
                });
            } else {
                diagnostics.add(InvalidPropertyName {
                    property: property.name.span,
                    node_type: S::NODE_TYPE.with_span(node.node_type.span),
                    expected_names: S::supported_properties()
                        .iter()
                        .filter_map(|p| p.name.as_exact())
                        .sorted()
                        .copied()
                        .collect(),
                });
            }

            continue;
        };

        if !property.doc_comments.is_empty() && !property_info.supports_doc_comments {
            let doc_comments = property
                .doc_comments
                .iter()
                .map(|dc| dc.span)
                .reduce(|x, y| x.to(y))
                .unwrap();

            diagnostics.add(IgnoredDocCommentOnProperty {
                doc_comments,
                property: property.name.span,
            });
        }

        // Get the discriminant and cast it to the static lifetime which is explicitly allowed in the rust docs
        let current_expression_type = unsafe {
            std::mem::transmute::<Discriminant<Expression<'src>>, Discriminant<Expression<'static>>>(
                mem::discriminant(&property.expression.value),
            )
        };

        let expression_supported =
            property_info
                .allowed_expression_types
                .iter()
                .any(|allowed_expression_type| {
                    current_expression_type == mem::discriminant(allowed_expression_type)
                });

        if !expression_supported {
            diagnostics.add(InvalidExpressionType {
                expression: property
                    .expression
                    .to_string()
                    .with_span(property.expression.span),
                node_type: S::NODE_TYPE.with_span(node.node_type.span),
                valid_expression_types: property_info
                    .allowed_expression_types
                    .iter()
                    .map(|e| e.to_string())
                    .collect(),
                valid_expression_values: property_info
                    .allowed_expression_types
                    .iter()
                    .map(|e| e.get_human_string())
                    .collect(),
            });
            continue;
        }

        error |= (property_info.setter)(SetterArgs {
            target_object: &mut target,
            property,
            node,
            diagnostics,
            sibling_objects: &mut sibling_objects,
        });

        if !property_info.multiple_allowed {
            possible_properties.remove(possible_properties.element_offset(property_info).unwrap());
            removed_properties.insert(property.name.val, property.name.span);
        }
    }

    for short_property in node.short_properties.iter() {
        let short_property_discriminant = discriminant(&short_property.value);

        let Some(property_info) = possible_properties.iter().find(|p| {
            p.name.as_short().is_some()
                && p.allowed_expression_types
                    .iter()
                    .map(discriminant)
                    .any(|ed| ed == short_property_discriminant)
        }) else {
            if let Some(original) = removed_short_properties
                .get(&short_property_discriminant)
                .copied()
            {
                diagnostics.add(DuplicateProperty {
                    original,
                    duplicate: short_property.span,
                });
            } else {
                diagnostics.add(InvalidShortProperty {
                    property: short_property.span,
                    node_type: S::NODE_TYPE.with_span(node.node_type.span),
                    got: short_property.to_string(),
                    expected: S::supported_properties()
                        .iter()
                        .filter_map(|p| {
                            p.name.as_short().map(|purpose| {
                                p.allowed_expression_types
                                    .iter()
                                    .map(|e| (e.to_string(), purpose.to_string()))
                            })
                        })
                        .flatten()
                        .sorted()
                        .collect(),
                });
            }

            continue;
        };

        error |= (property_info.setter)(SetterArgs {
            target_object: &mut target,
            property: &Property {
                doc_comments: Vec::new(),
                name: Ident::new("", short_property.span),
                expression: short_property.clone(),
            }
            .with_span(short_property.span),
            node,
            diagnostics,
            sibling_objects: &mut sibling_objects,
        });

        if !property_info.multiple_allowed {
            for allowed_expression in property_info.allowed_expression_types.iter() {
                removed_short_properties
                    .insert(discriminant(allowed_expression), short_property.span);
            }
            possible_properties.remove(possible_properties.element_offset(property_info).unwrap());
        }
    }

    // Required properties that haven't been seen
    let missing_properties = possible_properties
        .iter()
        .filter(|info| info.required)
        .collect::<Vec<_>>();

    if !missing_properties.is_empty() {
        for missing_info in missing_properties {
            diagnostics.add(MissingRequiredProperty {
                node_type: S::NODE_TYPE.with_span(node.node_type.span),
                property_name: match missing_info.name {
                    PropertyName::Exact(val) => val.to_string(),
                    PropertyName::Short(val) => val.to_string(),
                    _ => "*".to_string(),
                },
                short: matches!(missing_info.name, PropertyName::Short(_)),
                allowed_property_types: missing_info
                    .allowed_expression_types
                    .iter()
                    .map(|e| e.to_string())
                    .collect(),
            });
        }
        error = true;
    }

    // Sub nodes

    if let Some(supported_subnodes) = S::supported_subnodes() {
        for sub_node in node.sub_nodes.iter() {
            let sub_node_result = lower_node(
                sub_node,
                Some(S::NODE_TYPE.with_span(node.node_type.span)),
                None,
                supported_subnodes,
                diagnostics,
            );

            match sub_node_result {
                LowerResult::Manifest(_) => unreachable!(),
                LowerResult::Objects(object, siblings) => {
                    target.push_subnode(object);
                    sibling_objects.extend(siblings);
                }
                LowerResult::Error(siblings) => {
                    sibling_objects.extend(siblings);
                }
            }
        }
    } else if let Some(subnode) = node.sub_nodes.first() {
        diagnostics.add(InvalidSubnode {
            node_type: S::NODE_TYPE.with_span(node.node_type.span),
            subnode: subnode.span,
        });
    }

    // Make all sibling objects into subnodes if supported
    if let Some(supported_subnodes) = S::supported_subnodes() {
        for i in (0..sibling_objects.len()).rev() {
            if supported_subnodes.contains(&sibling_objects[i].node_type()) {
                target.push_subnode(sibling_objects.remove(i));
            }
        }
    }

    if !error {
        Ok((target, sibling_objects))
    } else {
        Err(sibling_objects)
    }
}

trait Shape: Default + 'static {
    const NODE_TYPE: NodeType;
    type NameIdentifierType: IdentifierType + Default;

    fn doc_comments(&mut self) -> &mut String;
    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>>;

    /// All the supported properties
    fn supported_properties() -> &'static [PropertyInfo<Self>];

    fn supported_subnodes() -> Option<&'static [NodeType]> {
        None
    }

    fn push_subnode(&mut self, _: Object) {
        unimplemented!()
    }

    /// If the shape requires a base type, Some is returned
    fn base_type(&mut self) -> Option<&mut Spanned<BaseType>> {
        None
    }

    fn conversion_type(&mut self) -> Option<&mut Option<TypeConversion>> {
        None
    }

    fn repeat(&mut self) -> Option<&mut Option<Repeat>> {
        None
    }

    fn span(&mut self) -> &mut Span;
}

struct PropertyInfo<T: ?Sized> {
    name: PropertyName<'static>,
    /// The types of expressions that are supported.
    /// Comparison is done using discriminants only.
    /// The values of the expressions are used for suggestions in diagnostics.
    allowed_expression_types: Cow<'static, [Expression<'static>]>,
    /// If true, multiple of these properties are allowed
    multiple_allowed: bool,
    /// If true, the property must be set by the user.
    /// Doesn't work well with [`Self::multiple_allowed`] set at the same time.
    required: bool,
    /// If false, a warning is emitted when the property has doc comments
    supports_doc_comments: bool,
    /// If setter returns true, there's an error
    setter: for<'a, 'src> fn(SetterArgs<'a, 'src, T>) -> bool,
}

impl<T: ?Sized> Clone for PropertyInfo<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            allowed_expression_types: self.allowed_expression_types.clone(),
            multiple_allowed: self.multiple_allowed,
            required: self.required,
            supports_doc_comments: self.supports_doc_comments,
            setter: self.setter,
        }
    }
}

struct SetterArgs<'a, 'src, T: ?Sized> {
    /// The target object that needs a property set
    target_object: &'a mut T,
    /// The property that needs to be set
    property: &'a Spanned<Property<'src>>,
    /// The node that's being parsed
    node: &'a Node<'src>,
    diagnostics: &'a mut Diagnostics,
    sibling_objects: &'a mut Vec<Object>,
}

#[derive(Clone, Copy)]
enum PropertyName<'a> {
    Exact(&'a str),
    Any,
    Short(&'a str),
}

impl<'a> PropertyName<'a> {
    fn as_exact(&self) -> Option<&&'a str> {
        if let Self::Exact(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_short(&self) -> Option<&&'a str> {
        if let Self::Short(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<'a> PartialEq for PropertyName<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(l0), Self::Exact(r0)) => l0 == r0,
            (Self::Short(l0), Self::Short(r0)) => l0 == r0,
            (Self::Exact(_), Self::Any) => true,
            (Self::Any, Self::Exact(_)) => true,
            _ => false,
        }
    }
}

const FIELD_SET_EXAMPLE: Node<'static> = Node {
    doc_comments: Vec::new(),
    node_type: device_driver_parser::Ident::new_no_span("fieldset"),
    name: device_driver_parser::Ident::new_no_span("MyFieldSet"),
    repeat: None,
    type_specifier: None,
    properties: Vec::new(),
    short_properties: Vec::new(),
    sub_nodes: Vec::new(),
    span: Span::empty(),
};

impl Shape for Manifest {
    const NODE_TYPE: NodeType = NodeType::Manifest;
    type NameIdentifierType = All;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Manifest>] = &[
            PropertyInfo {
                name: PropertyName::Exact("byte-order"),
                allowed_expression_types: Cow::Borrowed(&[Expression::ByteOrder(ByteOrder::LE)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.byte_order = Some(property.expression.as_byte_order().unwrap());
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("register-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.register_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("command-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.command_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("buffer-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.buffer_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("word-boundaries"),
                allowed_expression_types: Cow::Borrowed(&[Expression::String("bD:0B:_")]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.name_word_boundaries = Some(Boundary::defaults_from(
                        property.expression.as_string().unwrap(),
                    ));
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("register-address-mode"),
                allowed_expression_types: Cow::Borrowed(&[Expression::AddressMode(
                    AddressMode::Mapped,
                )]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    manifest.config.register_address_mode = Some(
                        property
                            .expression
                            .as_address_mode()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-bus"),
                allowed_expression_types: Cow::Borrowed(&[Expression::SvBus(SvBus::Native)]),
                multiple_allowed: true,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: manifest,
                             property,
                             ..
                         }| {
                    if let Some(val) = property.expression.as_sv_bus() {
                        manifest
                            .config
                            .sv_bus
                            .push(val.with_span(property.expression.span));
                    }
                    false
                },
            },
        ];
        MAP
    }

    fn supported_subnodes() -> Option<&'static [NodeType]> {
        Some(&[
            NodeType::Device,
            NodeType::FieldSet,
            NodeType::Enum,
            NodeType::Extern,
        ])
    }

    fn push_subnode(&mut self, object: Object) {
        self.objects.push(object);
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Device {
    const NODE_TYPE: NodeType = NodeType::Device;
    type NameIdentifierType = Type;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Device>] = &[
            PropertyInfo {
                name: PropertyName::Exact("byte-order"),
                allowed_expression_types: Cow::Borrowed(&[Expression::ByteOrder(ByteOrder::LE)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: dev,
                             property,
                             ..
                         }| {
                    dev.device_config.byte_order =
                        Some(property.expression.as_byte_order().unwrap());
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("register-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: dev,
                             property,
                             ..
                         }| {
                    dev.device_config.register_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("command-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: dev,
                             property,
                             ..
                         }| {
                    dev.device_config.command_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("buffer-address-type"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Integer(Integer::I32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: dev,
                             property,
                             ..
                         }| {
                    dev.device_config.buffer_address_type = Some(
                        property
                            .expression
                            .as_integer()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("word-boundaries"),
                allowed_expression_types: Cow::Borrowed(&[Expression::String("bD:0B:_")]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: dev,
                             property,
                             ..
                         }| {
                    dev.device_config.name_word_boundaries = Some(Boundary::defaults_from(
                        property.expression.as_string().unwrap(),
                    ));
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("register-address-mode"),
                allowed_expression_types: Cow::Borrowed(&[Expression::AddressMode(
                    AddressMode::Mapped,
                )]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    device.device_config.register_address_mode = Some(
                        property
                            .expression
                            .as_address_mode()
                            .unwrap()
                            .with_span(property.expression.span),
                    );
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-bus"),
                allowed_expression_types: Cow::Borrowed(&[Expression::SvBus(SvBus::Native)]),
                multiple_allowed: true,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if let Some(val) = property.expression.as_sv_bus() {
                        device
                            .device_config
                            .sv_bus
                            .push(val.with_span(property.expression.span));
                    }
                    false
                },
            },
            // SVA opt-in flags. Each `<flag>: allow` toggles one category in
            // the emitted `<dev>_sva` checker module. With none enabled, no
            // checker file is emitted.
            PropertyInfo {
                name: PropertyName::Exact("sv-assert-reset"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_assertions.reset = true;
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-assert-decode-mutex"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_assertions.decode_mutex = true;
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-assert-w1c"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_assertions.w1c = true;
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-assert-ro-invariance"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_assertions.ro_invariance = true;
                    }
                    false
                },
            },
            // UVM RAL opt-out. The RAL package is emitted by default
            // (item 120 from the SV-backend roadmap — verification teams
            // normally always want it). `sv-no-ral: allow` opts out for
            // SW-only consumers who don't care about UVM artifacts.
            // `sv-ral: allow` is retained as an explicit no-op for DSL
            // sources that already pinned the value.
            PropertyInfo {
                name: PropertyName::Exact("sv-ral"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_ral = true;
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-no-ral"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.sv_ral = false;
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("sv-hdl-path-prefix"),
                allowed_expression_types: Cow::Borrowed(&[Expression::String("u_top.u_regs")]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    device.device_config.sv_hdl_path_prefix =
                        property.expression.as_string().map(str::to_string);
                    false
                },
            },
            // CPUIF data-bus width in bits. When omitted, codegen falls back
            // to the `--option sv-data-width=N` CLI override, then 32.
            PropertyInfo {
                name: PropertyName::Exact("sv-data-width"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(32)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    // Capture span unconditionally and defer {32, 64}
                    // validation to `passes::bus_compat_checked`. Silent
                    // drop here was bad UX: the user saw `sv-data-width: 7`
                    // accepted and a regblock generated at 32 bits.
                    if let Some(n) = property.expression.as_number() {
                        let value = n.clamp(0, u32::MAX as i128) as u32;
                        device.device_config.sv_data_width =
                            Some(value.with_span(property.expression.span));
                    }
                    false
                },
            },
            // Suppress the root `irq` OR-reduce output. Default is on (root
            // `irq` is emitted when ungrouped intr fields exist). Per-group
            // `irq_<group>` outputs are unaffected by this flag.
            PropertyInfo {
                name: PropertyName::Exact("intr-no-aggregate"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if matches!(property.expression.value, Expression::Allow) {
                        device.device_config.intr_aggregate = Some(false);
                    }
                    false
                },
            },
            // Base address for synthesized per-group intr enable/mask
            // companion registers. Each distinct group occupies one
            // CPUIF-data-width slot starting at the base.
            PropertyInfo {
                name: PropertyName::Exact("intr-enable-address-base"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number() {
                        device.device_config.intr_enable_address_base = Some(n);
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("intr-mask-address-base"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: device,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number() {
                        device.device_config.intr_mask_address_base = Some(n);
                    }
                    false
                },
            },
        ];
        MAP
    }

    fn supported_subnodes() -> Option<&'static [NodeType]> {
        Some(&[
            NodeType::Block,
            NodeType::Register,
            NodeType::Command,
            NodeType::Buffer,
            NodeType::FieldSet,
            NodeType::Enum,
            NodeType::Extern,
        ])
    }

    fn push_subnode(&mut self, object: Object) {
        self.objects.push(object);
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Block {
    const NODE_TYPE: NodeType = NodeType::Block;
    type NameIdentifierType = All;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Block>] = &[PropertyInfo {
            name: PropertyName::Exact("address-offset"),
            allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
            multiple_allowed: false,
            required: true,
            supports_doc_comments: false,
            setter: |SetterArgs {
                         target_object: block,
                         property,
                         ..
                     }| {
                block.address_offset = property
                    .expression
                    .as_number()
                    .unwrap()
                    .with_span(property.expression.span);
                false
            },
        }];
        MAP
    }

    fn supported_subnodes() -> Option<&'static [NodeType]> {
        Some(&[
            NodeType::Block,
            NodeType::Register,
            NodeType::Command,
            NodeType::Buffer,
            NodeType::FieldSet,
            NodeType::Enum,
            NodeType::Extern,
        ])
    }

    fn push_subnode(&mut self, object: Object) {
        self.objects.push(object);
    }

    fn repeat(&mut self) -> Option<&mut Option<Repeat>> {
        Some(&mut self.repeat)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Register {
    const NODE_TYPE: NodeType = NodeType::Register;
    type NameIdentifierType = Operation;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: LazyLock<Vec<PropertyInfo<Register>>> = LazyLock::new(|| {
            [
                PropertyInfo {
                    name: PropertyName::Exact("address"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                    multiple_allowed: false,
                    required: true,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Register> {
                                 target_object: r,
                                 property,
                                 ..
                             }| {
                        r.address = property
                            .expression
                            .as_number()
                            .unwrap()
                            .with_span(property.expression.span);
                        false
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("access"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Access(Access::RW)]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Register> {
                                 target_object: r,
                                 property,
                                 ..
                             }| {
                        r.access = property.expression.as_access().unwrap();
                        false
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("address-overlap"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Register> {
                                 target_object: r, ..
                             }| {
                        r.allow_address_overlap = true;
                        false
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("reset"),
                    allowed_expression_types: Cow::Owned(vec![
                        Expression::ByteArray(vec![12, 34]),
                        Expression::Number(1234),
                    ]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Register> {
                                 target_object: r,
                                 property,
                                 diagnostics,
                                 ..
                             }| match &property.expression.value {
                        Expression::Number(num) => match u128::try_from(*num) {
                            Ok(num) => {
                                r.reset_value = Some(
                                    ResetValue::Integer(num).with_span(property.expression.span),
                                );
                                false
                            }
                            Err(_) => {
                                diagnostics.add(ResetValueNegative {
                                    reset_value: property.expression.span,
                                });
                                true
                            }
                        },
                        Expression::ByteArray(bytes) => {
                            r.reset_value = Some(
                                ResetValue::Array(bytes.to_vec())
                                    .with_span(property.expression.span),
                            );
                            false
                        }
                        _ => unreachable!(),
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("fields"),
                    allowed_expression_types: Cow::Owned(vec![
                        Expression::TypeReference(device_driver_parser::Ident::new_no_span(
                            "MyFieldset",
                        )),
                        Expression::SubNode(Box::new(FIELD_SET_EXAMPLE)),
                    ]),
                    multiple_allowed: false,
                    required: true,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Register> {
                                 target_object: r,
                                 property,
                                 node,
                                 diagnostics,
                                 sibling_objects,
                             }| {
                        match &property.expression.value {
                            Expression::TypeReference(ident) => {
                                r.field_set_ref =
                                    IdentifierRef::new(ident.val.into()).with_span(ident.span);
                                false
                            }
                            Expression::SubNode(sub_node) => {
                                let result = lower_node(
                                    sub_node,
                                    Some(NodeType::Register.with_span(node.node_type.span)),
                                    Some(Ident::new(r.name.original(), r.name.span)),
                                    &[NodeType::FieldSet],
                                    diagnostics,
                                );

                                match result {
                                    LowerResult::Objects(fs, fs_siblings) => {
                                        r.field_set_ref = fs
                                            .name()
                                            .clone()
                                            // This should always be a fieldset is a Type identifier
                                            .cast_assert()
                                            .take_ref()
                                            .with_span(fs.name_span());
                                        sibling_objects.push(fs);
                                        sibling_objects.extend(fs_siblings);
                                        false
                                    }
                                    LowerResult::Error(fs_siblings) => {
                                        sibling_objects.extend(fs_siblings);
                                        true
                                    }
                                    LowerResult::Manifest(_) => unreachable!(),
                                }
                            }
                            _ => unreachable!(),
                        }
                    },
                },
                // SystemVerilog-target reserved-bit policy.
                PropertyInfo {
                    name: PropertyName::Exact("reserved-behavior"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::ReservedBehavior(
                        ReservedBehavior::RoZero,
                    )]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs {
                                 target_object: r,
                                 property,
                                 ..
                             }| {
                        if let Some(rb) = property.expression.as_reserved_behavior() {
                            r.reserved_behavior = rb;
                        }
                        false
                    },
                },
                // SystemVerilog-target opt-in. With `external: allow`, the
                // register has no storage flop; the regblock exposes
                // `<reg>_ext_*` ports on hwif so user RTL drives the read
                // data and observes write transactions directly.
                PropertyInfo {
                    name: PropertyName::Exact("external"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs {
                                 target_object: r,
                                 property,
                                 ..
                             }| {
                        if matches!(property.expression.value, Expression::Allow) {
                            r.external = true;
                        }
                        false
                    },
                },
            ]
            .into()
        });
        &MAP
    }

    fn repeat(&mut self) -> Option<&mut Option<Repeat>> {
        Some(&mut self.repeat)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for FieldSet {
    const NODE_TYPE: NodeType = NodeType::FieldSet;
    type NameIdentifierType = Type;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<FieldSet>] = &[
            PropertyInfo {
                name: PropertyName::Exact("size-bytes"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(8)]),
                multiple_allowed: false,
                required: true,
                supports_doc_comments: false,
                setter: |SetterArgs::<FieldSet> {
                             target_object: fs,
                             property,
                             node: fs_node,
                             diagnostics,
                             ..
                         }| match u32::try_from(
                    property.expression.as_number().unwrap(),
                ) {
                    Ok(size_bytes) if size_bytes <= 0x10_0000 => {
                        fs.size_bytes = size_bytes.with_span(property.expression.span);
                        false
                    }
                    _ => {
                        diagnostics.add(SizeBytesTooLarge {
                            value: property.expression.span,
                            field_set: fs_node.span,
                        });
                        true
                    }
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("byte-order"),
                allowed_expression_types: Cow::Borrowed(&[Expression::ByteOrder(ByteOrder::LE)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<FieldSet> {
                             target_object: fs,
                             property,
                             ..
                         }| {
                    fs.byte_order = Some(property.expression.as_byte_order().unwrap());
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("bit-overlap"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<FieldSet> {
                             target_object: fs, ..
                         }| {
                    fs.allow_bit_overlap = true;
                    false
                },
            },
        ];
        MAP
    }

    fn supported_subnodes() -> Option<&'static [NodeType]> {
        Some(&[NodeType::Field])
    }

    fn push_subnode(&mut self, object: Object) {
        let Object::Field(field) = object else {
            unreachable!("{object:?}")
        };
        self.fields.push(field);
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Extern {
    const NODE_TYPE: NodeType = NodeType::Extern;
    type NameIdentifierType = Type;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Extern>] = &[
            PropertyInfo {
                name: PropertyName::Exact("infallible"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<Extern> {
                             target_object: ext, ..
                         }| {
                    ext.supports_infallible = true;
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("size-bits"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(8)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<Extern> {
                             target_object: ext,
                             property,
                             diagnostics,
                             node,
                             ..
                         }| match u64::try_from(
                    property.expression.as_number().unwrap(),
                ) {
                    Ok(size_bits) => {
                        ext.size_bits = Some(size_bits.with_span(property.expression.span));
                        false
                    }
                    _ => {
                        diagnostics.add(ExternInvalidSizeBits {
                            extern_name: node.name.span,
                            size_bits: property.expression.span,
                            reason: "value must be in the range 0..2^64".into(),
                        });
                        true
                    }
                },
            },
        ];
        MAP
    }

    fn base_type(&mut self) -> Option<&mut Spanned<BaseType>> {
        Some(&mut self.base_type)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Buffer {
    const NODE_TYPE: NodeType = NodeType::Buffer;
    type NameIdentifierType = Operation;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Buffer>] = &[
            PropertyInfo {
                name: PropertyName::Exact("access"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Access(Access::RW)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<Buffer> {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    buf.access = property.expression.as_access().unwrap();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("address"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: true,
                supports_doc_comments: false,
                setter: |SetterArgs::<Buffer> {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    buf.address = property
                        .expression
                        .as_number()
                        .unwrap()
                        .with_span(property.expression.span);
                    false
                },
            },
            // SystemVerilog target opt-in. `hw-kind: fifo` (the only v1 value)
            // wires bus push/pop strobes on the buffer's address. The regblock
            // intentionally does NOT instantiate the FIFO storage — user RTL
            // plugs its own FIFO into the hwif handshake signals. Rust target
            // ignores both properties.
            PropertyInfo {
                name: PropertyName::Exact("hw-kind"),
                allowed_expression_types: Cow::Borrowed(&[Expression::HwKind(HwKind::Fifo)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    buf.hw_kind = property.expression.as_hw_kind();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("depth"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number()
                        && n > 0
                        && n <= u32::MAX as i128
                    {
                        buf.depth = Some(n as u32);
                    }
                    false
                },
            },
            // Address at which the future auto-synthesized companion
            // status register will live. Held in MIR + LIR + carried
            // verbatim into hwif doc comments; synthesis to come.
            PropertyInfo {
                name: PropertyName::Exact("status-address"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number() {
                        buf.status_address = Some(n);
                    }
                    false
                },
            },
            // Streaming-mode multi-word access hint. Informational only
            // in v1.
            PropertyInfo {
                name: PropertyName::Exact("words"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: buf,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number()
                        && n > 0
                        && n <= u32::MAX as i128
                    {
                        buf.words = Some(n as u32);
                    }
                    false
                },
            },
        ];
        MAP
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Enum {
    const NODE_TYPE: NodeType = NodeType::Enum;
    type NameIdentifierType = Type;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Enum>] = &[PropertyInfo {
            name: PropertyName::Any,
            allowed_expression_types: Cow::Borrowed(&[
                Expression::Auto,
                Expression::Number(0),
                Expression::DefaultNumber(Some(0)),
                Expression::DefaultNumber(None),
                Expression::CatchAllNumber(Some(0)),
                Expression::CatchAllNumber(None),
            ]),
            multiple_allowed: true,
            required: false,
            supports_doc_comments: true,
            setter: |SetterArgs::<Enum> {
                         target_object: enum_value,
                         property,
                         diagnostics,
                         ..
                     }| {
                let identifier = match Identifier::try_parse(property.name.val) {
                    Ok(identifier) => identifier,
                    Err(e) => {
                        diagnostics.add(InvalidIdentifier {
                            error: e,
                            identifier: property.name.span,
                        });
                        return true;
                    }
                };

                enum_value.variants.push(EnumVariant {
                    description: property.doc_comments.iter().map(|c| c.value).join("\n"),
                    name: identifier.with_span(property.name.span),
                    value: match &property.expression.value {
                        Expression::Number(num) => EnumValue::Specified(*num),
                        Expression::DefaultNumber(Some(num)) => EnumValue::Default(*num),
                        Expression::DefaultNumber(None) => EnumValue::UnspecifiedDefault,
                        Expression::CatchAllNumber(Some(num)) => EnumValue::CatchAll(*num),
                        Expression::CatchAllNumber(None) => EnumValue::UnspecifiedCatchAll,
                        Expression::Auto => EnumValue::Unspecified,
                        _ => unreachable!(),
                    },
                    span: property.span,
                });
                false
            },
        }];
        MAP
    }

    fn base_type(&mut self) -> Option<&mut Spanned<BaseType>> {
        Some(&mut self.base_type)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Command {
    const NODE_TYPE: NodeType = NodeType::Command;
    type NameIdentifierType = Operation;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: LazyLock<Vec<PropertyInfo<Command>>> = LazyLock::new(|| {
            [
                PropertyInfo {
                    name: PropertyName::Exact("address"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                    multiple_allowed: false,
                    required: true,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Command> {
                                 target_object: command,
                                 property,
                                 ..
                             }| {
                        command.address = property
                            .expression
                            .as_number()
                            .unwrap()
                            .with_span(property.expression.span);
                        false
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("address-overlap"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Command> {
                                 target_object: command,
                                 ..
                             }| {
                        command.allow_address_overlap = true;
                        false
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("fields-in"),
                    allowed_expression_types: Cow::Owned(vec![
                        Expression::TypeReference(device_driver_parser::Ident::new_no_span(
                            "MyFieldset",
                        )),
                        Expression::SubNode(Box::new(FIELD_SET_EXAMPLE)),
                    ]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Command> {
                                 target_object: command,
                                 property,
                                 node,
                                 diagnostics,
                                 sibling_objects,
                             }| {
                        match &property.expression.value {
                            Expression::TypeReference(ident) => {
                                command.field_set_ref_in = Some(
                                    IdentifierRef::new(ident.val.into()).with_span(ident.span),
                                );
                                false
                            }
                            Expression::SubNode(sub_node) => {
                                let result = lower_node(
                                    sub_node,
                                    Some(NodeType::Register.with_span(node.node_type.span)),
                                    Some(Ident::new(command.name.original(), command.name.span)),
                                    &[NodeType::FieldSet],
                                    diagnostics,
                                );

                                match result {
                                    LowerResult::Objects(fs, fs_siblings) => {
                                        command.field_set_ref_in = Some(
                                            fs.name()
                                                .clone()
                                                // Always a fieldset, so should be fine
                                                .cast_assert()
                                                .take_ref()
                                                .with_span(fs.name_span()),
                                        );
                                        sibling_objects.push(fs);
                                        sibling_objects.extend(fs_siblings);
                                        false
                                    }
                                    LowerResult::Error(fs_siblings) => {
                                        sibling_objects.extend(fs_siblings);
                                        true
                                    }
                                    LowerResult::Manifest(_) => unreachable!(),
                                }
                            }
                            _ => unreachable!(),
                        }
                    },
                },
                PropertyInfo {
                    name: PropertyName::Exact("fields-out"),
                    allowed_expression_types: Cow::Owned(vec![
                        Expression::TypeReference(device_driver_parser::Ident::new_no_span(
                            "MyFieldset",
                        )),
                        Expression::SubNode(Box::new(FIELD_SET_EXAMPLE)),
                    ]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs::<Command> {
                                 target_object: command,
                                 property,
                                 node,
                                 diagnostics,
                                 sibling_objects,
                             }| {
                        match &property.expression.value {
                            Expression::TypeReference(ident) => {
                                command.field_set_ref_out = Some(
                                    IdentifierRef::new(ident.val.into()).with_span(ident.span),
                                );
                                false
                            }
                            Expression::SubNode(sub_node) => {
                                let result = lower_node(
                                    sub_node,
                                    Some(NodeType::Register.with_span(node.node_type.span)),
                                    Some(Ident::new(command.name.original(), command.name.span)),
                                    &[NodeType::FieldSet],
                                    diagnostics,
                                );

                                match result {
                                    LowerResult::Objects(fs, fs_siblings) => {
                                        command.field_set_ref_out = Some(
                                            fs.name()
                                                .clone()
                                                // Always a fieldset, so should be fine
                                                .cast_assert()
                                                .take_ref()
                                                .with_span(fs.name_span()),
                                        );
                                        sibling_objects.push(fs);
                                        sibling_objects.extend(fs_siblings);
                                        false
                                    }
                                    LowerResult::Error(fs_siblings) => {
                                        sibling_objects.extend(fs_siblings);
                                        true
                                    }
                                    LowerResult::Manifest(_) => unreachable!(),
                                }
                            }
                            _ => unreachable!(),
                        }
                    },
                },
                // SystemVerilog target opt-in. With `hw-handshake: strobe`,
                // the regblock emits `cmd_<n>_strobe` + `cmd_<n>_in` (out
                // of the regs module) and, if `fields-out` is set, also
                // latches `cmd_<n>_resp` (into the regs module) for read
                // back. Rust target ignores this property.
                PropertyInfo {
                    name: PropertyName::Exact("hw-handshake"),
                    allowed_expression_types: Cow::Borrowed(&[Expression::HwHandshake(
                        HwHandshake::Strobe,
                    )]),
                    multiple_allowed: false,
                    required: false,
                    supports_doc_comments: false,
                    setter: |SetterArgs {
                                 target_object: command,
                                 property,
                                 ..
                             }| {
                        command.hw_handshake = property.expression.as_hw_handshake();
                        false
                    },
                },
            ]
            .into()
        });
        &MAP
    }

    fn repeat(&mut self) -> Option<&mut Option<Repeat>> {
        Some(&mut self.repeat)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl Shape for Field {
    const NODE_TYPE: NodeType = NodeType::Field;
    type NameIdentifierType = All;

    fn doc_comments(&mut self) -> &mut String {
        &mut self.description
    }

    fn name(&mut self) -> &mut Spanned<Identifier<Self::NameIdentifierType>> {
        &mut self.name
    }

    fn supported_properties() -> &'static [PropertyInfo<Self>] {
        static MAP: &[PropertyInfo<Field>] = &[
            PropertyInfo {
                name: PropertyName::Short("address"),
                allowed_expression_types: Cow::Borrowed(&[
                    Expression::Number(0),
                    Expression::AddressRange { end: 8, start: 0 },
                ]),
                multiple_allowed: false,
                required: true,
                supports_doc_comments: false,
                setter: |SetterArgs::<Field> {
                             target_object: field,
                             property,
                             diagnostics,
                             ..
                         }| {
                    let u32_range = 0..=u32::MAX as i128;

                    field.field_address = match property.expression.value {
                        Expression::AddressRange { end, start }
                            if u32_range.contains(&end) && u32_range.contains(&start) =>
                        {
                            if end < start {
                                diagnostics.add(FieldAddressWrongOrder {
                                    address: property.expression.span,
                                    end,
                                    start,
                                });
                                return true;
                            }

                            AddressRange {
                                start: start.try_into().unwrap(),
                                end: end.try_into().unwrap(),
                            }
                        }
                        Expression::Number(num) if u32_range.contains(&num) => AddressRange {
                            start: num.try_into().unwrap(),
                            end: num.try_into().unwrap(),
                        },
                        Expression::AddressRange { .. } | Expression::Number(_) => {
                            diagnostics.add(FieldAddressOutOfRange {
                                field_address: property.expression.span,
                            });
                            return true;
                        }
                        _ => unreachable!(),
                    }
                    .with_span(property.expression.span);
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Short("access"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Access(Access::RW)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs::<Field> {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.access = property.expression.as_access().unwrap();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("on-write"),
                allowed_expression_types: Cow::Borrowed(&[Expression::OnWrite(OnWrite::Store)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.on_write = property.expression.as_on_write();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("hw-access"),
                // Reuses the sw Access token (RW/RO/WO); the setter maps each
                // variant to the corresponding HwAccess. To express "HW has no
                // observation port" (rare), omit the property entirely — the
                // default `HwAccess::RO` is what every existing register gets.
                allowed_expression_types: Cow::Borrowed(&[Expression::Access(Access::RW)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.hw_access = property.expression.as_access().map(HwAccess::from);
                    false
                },
            },
            // HW-side strobe modifiers — `<property>: allow` opts each in.
            // `hw-clr` and `hw-set` wire a single-bit input that forces the
            // field to all-zeros / all-ones for one cycle. `singlepulse` makes
            // the field auto-clear the cycle after any non-zero state.
            PropertyInfo {
                name: PropertyName::Exact("hw-clr"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.hw_clr = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("hw-set"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.hw_set = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("singlepulse"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.singlepulse = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            // Override HW vs SW write precedence for this field. Default `Hw`
            // matches SystemRDL and is what every prior milestone bakes in.
            PropertyInfo {
                name: PropertyName::Exact("precedence"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Precedence(Precedence::Hw)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.precedence = property.expression.as_precedence();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("on-read"),
                // Reuses the OnWrite token type at the lexer/parser level; the
                // setter narrows to OnRead and rejects `toggle`.
                allowed_expression_types: Cow::Borrowed(&[Expression::OnWrite(OnWrite::Store)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             node,
                             diagnostics,
                             ..
                         }| {
                    let raw = property.expression.as_on_write().unwrap();
                    field.on_read = match raw {
                        OnWrite::Store => Some(OnRead::Store),
                        OnWrite::Clear => Some(OnRead::Clear),
                        OnWrite::Set => Some(OnRead::Set),
                        OnWrite::Toggle => {
                            diagnostics.add(
                                device_driver_diagnostics::errors::OnReadToggleNotAllowed {
                                    property_span: property.expression.span,
                                    field_set_context: node.name.span,
                                },
                            );
                            None
                        }
                    };
                    false
                },
            },
            // Interrupt-source field: mark the field as an IRQ source with the
            // given trigger semantics. Emits a raw input `hwif_in.<f>_intr`,
            // edge-detect (where applicable), and OR-reduces sticky storage
            // into `irq_<group>` (or root `irq` if no `intr-group` set).
            PropertyInfo {
                name: PropertyName::Exact("intr-trigger"),
                allowed_expression_types: Cow::Borrowed(&[Expression::IntrTrigger(
                    IntrTrigger::Level,
                )]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.intr_trigger = property.expression.as_intr_trigger();
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("intr-group"),
                allowed_expression_types: Cow::Borrowed(&[Expression::String("group")]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.intr_group = property.expression.as_string().map(str::to_string);
                    false
                },
            },
            // `intr-sticky: allow` opts the field into latched behaviour
            // (default true once `intr-trigger` is set; explicit `false`
            // disables — exposed for testability rather than a useful
            // production pattern).
            PropertyInfo {
                name: PropertyName::Exact("intr-sticky"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.intr_sticky = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("intr-enable"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.intr_enable = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("intr-mask"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Allow]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    field.intr_mask = matches!(property.expression.value, Expression::Allow);
                    false
                },
            },
            // Explicit companion bit positions. Without these, the LIR
            // synthesis pass falls back to declaration order — fine
            // pre-1.0 but ABI-fragile once anyone depends on a fixed
            // layout. The pair `intr-enable: allow, intr-enable-bit: N`
            // pins the bit to N in the synthesized
            // `<group>_intr_enable` register. Mirror semantics for
            // `intr-mask-bit`. The LIR collision check enforces
            // uniqueness within a (group, side).
            PropertyInfo {
                name: PropertyName::Exact("intr-enable-bit"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number() {
                        let value = n.clamp(0, u32::MAX as i128) as u32;
                        field.intr_enable_bit = Some(value.with_span(property.expression.span));
                    }
                    false
                },
            },
            PropertyInfo {
                name: PropertyName::Exact("intr-mask-bit"),
                allowed_expression_types: Cow::Borrowed(&[Expression::Number(0)]),
                multiple_allowed: false,
                required: false,
                supports_doc_comments: false,
                setter: |SetterArgs {
                             target_object: field,
                             property,
                             ..
                         }| {
                    if let Some(n) = property.expression.as_number() {
                        let value = n.clamp(0, u32::MAX as i128) as u32;
                        field.intr_mask_bit = Some(value.with_span(property.expression.span));
                    }
                    false
                },
            },
        ];
        MAP
    }

    fn base_type(&mut self) -> Option<&mut Spanned<BaseType>> {
        Some(&mut self.base_type)
    }

    fn conversion_type(&mut self) -> Option<&mut Option<TypeConversion>> {
        Some(&mut self.field_conversion)
    }

    fn repeat(&mut self) -> Option<&mut Option<Repeat>> {
        Some(&mut self.repeat)
    }

    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}
