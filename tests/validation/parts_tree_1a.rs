//! Parser test for `01-Parts Tree/1a-Parts Tree.sysml`.

use std::path::Path;
use sysml_parser::ast::{
    AttributeBody, AttributeDef, AttributeUsage, Expression, Identification, Import, Node,
    Package, PackageBody, PackageBodyElement, PartDef, PartDefBody, PartDefBodyElement, PartUsage,
    PartUsageBody, PartUsageBodyElement, RootElement, RootNamespace, Span, Visibility,
};
use sysml_parser::parse;

fn id(name: &str) -> Identification {
    Identification {
        short_name: None,
        name: Some(name.to_string()),
    }
}

fn n<T>(v: T) -> Node<T> {
    Node::new(Span::dummy(), v)
}

/// 1750 [kg]
fn expr_1750_kg() -> Node<Expression> {
    n(Expression::LiteralWithUnit {
        value: Box::new(n(Expression::LiteralInteger(1750))),
        unit: Box::new(n(Expression::Bracket(Box::new(n(Expression::FeatureRef("kg".to_string())))))),
    })
}

/// 2000 [kg]
fn expr_2000_kg() -> Node<Expression> {
    n(Expression::LiteralWithUnit {
        value: Box::new(n(Expression::LiteralInteger(2000))),
        unit: Box::new(n(Expression::Bracket(Box::new(n(Expression::FeatureRef("kg".to_string())))))),
    })
}

/// frontWheel#(1)
fn expr_front_wheel_1() -> Node<Expression> {
    n(Expression::Index {
        base: Box::new(n(Expression::FeatureRef("frontWheel".to_string()))),
        index: Box::new(n(Expression::LiteralInteger(1))),
    })
}

/// frontWheel#(2)
fn expr_front_wheel_2() -> Node<Expression> {
    n(Expression::Index {
        base: Box::new(n(Expression::FeatureRef("frontWheel".to_string()))),
        index: Box::new(n(Expression::LiteralInteger(2))),
    })
}

/// rearWheel#(1)
fn expr_rear_wheel_1() -> Node<Expression> {
    n(Expression::Index {
        base: Box::new(n(Expression::FeatureRef("rearWheel".to_string()))),
        index: Box::new(n(Expression::LiteralInteger(1))),
    })
}

/// rearWheel#(2)
fn expr_rear_wheel_2() -> Node<Expression> {
    n(Expression::Index {
        base: Box::new(n(Expression::FeatureRef("rearWheel".to_string()))),
        index: Box::new(n(Expression::LiteralInteger(2))),
    })
}

/// Expected AST for `1a-Parts Tree.sysml`: full structure with package, import, part def, part usage.
fn expected_ast() -> RootNamespace {
    RootNamespace {
        elements: vec![n(RootElement::Package(n(Package {
            identification: id("1a-Parts Tree"),
            body: PackageBody::Brace {
                elements: vec![
                    n(PackageBodyElement::Import(n(Import {
                        visibility: Some(Visibility::Private),
                        is_import_all: false,
target: "SI::kg".to_string(),
                    is_recursive: false,
                    filter_members: None,
                }))),
                    n(PackageBodyElement::Package(n(Package {
                        identification: id("Definitions"),
                        body: PackageBody::Brace {
                            elements: vec![
                                n(PackageBodyElement::PartDef(n(PartDef {
                                    identification: id("Vehicle"),
                                    specializes: None,
                                    body: PartDefBody::Brace {
                                        elements: vec![n(PartDefBodyElement::AttributeDef(n(
                                            AttributeDef {
                                                name: "mass".to_string(),
                                                typing: Some("ISQ::mass".to_string()),
                                                body: AttributeBody::Brace,
                                            },
                                        )))],
                                    },
                                }))),
                                n(PackageBodyElement::PartDef(n(PartDef {
                                    identification: id("AxleAssembly"),
                                    specializes: None,
                                    body: PartDefBody::Semicolon,
                                }))),
                                n(PackageBodyElement::PartDef(n(PartDef {
                                    identification: id("Axle"),
                                    specializes: None,
                                    body: PartDefBody::Brace {
                                        elements: vec![n(PartDefBodyElement::AttributeDef(n(
                                            AttributeDef {
                                                name: "mass".to_string(),
                                                typing: Some("ISQ::mass".to_string()),
                                                body: AttributeBody::Semicolon,
                                            },
                                        )))],
                                    },
                                }))),
                                n(PackageBodyElement::PartDef(n(PartDef {
                                    identification: id("FrontAxle"),
                                    specializes: Some("Axle".to_string()),
                                    body: PartDefBody::Brace {
                                        elements: vec![n(PartDefBodyElement::AttributeDef(n(
                                            AttributeDef {
                                                name: "steeringAngle".to_string(),
                                                typing: Some("ScalarValues::Real".to_string()),
                                                body: AttributeBody::Semicolon,
                                            },
                                        )))],
                                    },
                                }))),
                                n(PackageBodyElement::PartDef(n(PartDef {
                                    identification: id("Wheel"),
                                    specializes: None,
                                    body: PartDefBody::Semicolon,
                                }))),
                            ],
                        },
                    }))),
                    n(PackageBodyElement::Package(n(Package {
                        identification: id("Usages"),
                        body: PackageBody::Brace {
                            elements: vec![
                                n(PackageBodyElement::Import(n(Import {
                                    visibility: Some(Visibility::Private),
                                    is_import_all: true,
                                    target: "Definitions::*".to_string(),
                                    is_recursive: false,
                                    filter_members: None,
                                }))),
                                n(PackageBodyElement::PartUsage(n(part_vehicle1()))),
                                n(PackageBodyElement::PartUsage(n(part_vehicle1_c1()))),
                            ],
                        },
                    }))),
                ],
            },
        })))],
    }
}

fn part_vehicle1() -> PartUsage {
    PartUsage {
        name: "vehicle1".to_string(),
        type_name: "Vehicle".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::AttributeUsage(n(AttributeUsage {
                    name: "mass".to_string(),
                    redefines: Some("Vehicle::mass".to_string()),
                    value: Some(expr_1750_kg()),
                    body: AttributeBody::Brace,
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "frontAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })))),
                        ],
                    },
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "rearAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })))),
                        ],
                    },
                })))),
            ],
        },
    }
}

fn part_vehicle1_c1() -> PartUsage {
    PartUsage {
        name: "vehicle1_c1".to_string(),
        type_name: "Vehicle".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::AttributeUsage(n(AttributeUsage {
                    name: "mass".to_string(),
                    redefines: Some("Vehicle::mass".to_string()),
                    value: Some(expr_2000_kg()),
                    body: AttributeBody::Brace,
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "frontAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontAxle".to_string(),
                                type_name: "FrontAxle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontWheel_1".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "frontWheel".to_string(),
                                    Some(expr_front_wheel_1()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "frontWheel_2".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "frontWheel".to_string(),
                                    Some(expr_front_wheel_2()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })))),
                        ],
                    },
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "rearAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearWheel_1".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "rearWheel".to_string(),
                                    Some(expr_rear_wheel_1()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })))),
                            n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                                name: "rearWheel_2".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "rearWheel".to_string(),
                                    Some(expr_rear_wheel_2()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })))),
                        ],
                    },
                })))),
            ],
        },
    }
}

/// Fixture path for a SysML file under sysml-v2-release/sysml/src/validation/.
fn validation_fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sysml-v2-release")
        .join("sysml")
        .join("src")
        .join("validation")
        .join(relative)
}

#[test]
fn test_parse_1a_parts_tree() {
    super::init_log();
    let path = validation_fixture_path("01-Parts Tree").join("1a-Parts Tree.sysml");
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    log::debug!("input len: {} bytes, first 200 chars: {:?}", input.len(), input.chars().take(200).collect::<String>());
    let result = match parse(&input) {
        Ok(r) => r,
        Err(e) => {
            log::error!("parse failed: {}", e);
            log::error!("input len: {} bytes", input.len());
            log::error!("first 300 chars: {:?}", input.chars().take(300).collect::<String>());
            log::error!("first 100 bytes: {:?}", input.bytes().take(100).collect::<Vec<_>>());
            panic!("parse should succeed for 1a-Parts Tree.sysml: {}", e);
        }
    };
    let expected = expected_ast();
    assert_eq!(
        result, expected,
        "parsed AST should match expected for 1a-Parts Tree.sysml"
    );
}
