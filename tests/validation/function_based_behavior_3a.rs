//! Parser test for `03-Function-based Behavior/3a-Function-based Behavior-1.sysml`.

use std::path::Path;
use sysml_parser::ast::{
    ActionDef, ActionDefBody, ActionDefBodyElement, ActionUsage, ActionUsageBody, ActionUsageBodyElement, AliasBody,
    AliasDef, AttributeBody, AttributeDef, Bind, ConnectBody, Expression, FirstMergeBody, FirstStmt,
    Flow, Identification, Import, InOut, InOutDecl, MergeStmt, Node, Package, PackageBody,
    PackageBodyElement, RootElement, RootNamespace, Span, Visibility,
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

fn expr_path(path: &str) -> Node<Expression> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut expr = Expression::FeatureRef(segments[0].to_string());
    for seg in segments.iter().skip(1) {
        expr = Expression::MemberAccess(Box::new(n(expr)), (*seg).to_string());
    }
    n(expr)
}

fn expected_ast() -> RootNamespace {
    RootNamespace {
        elements: vec![n(RootElement::Package(n(Package {
            identification: id("3a-Function-based Behavior-1"),
            body: PackageBody::Brace {
                elements: vec![
                    n(PackageBodyElement::Import(n(Import {
                        visibility: Some(Visibility::Public),
                        is_import_all: true,
target: "Definitions::*".to_string(),
                    is_recursive: false,
                    filter_members: None,
                }))),
                n(PackageBodyElement::Import(n(Import {
                    visibility: Some(Visibility::Public),
                    is_import_all: true,
                    target: "Usages::*".to_string(),
                    is_recursive: false,
                    filter_members: None,
                }))),
                    n(PackageBodyElement::Package(n(definitions_package()))),
                    n(PackageBodyElement::Package(n(usages_package()))),
                ],
            },
        })))],
    }
}

fn definitions_package() -> Package {
    Package {
        identification: id("Definitions"),
        body: PackageBody::Brace {
            elements: vec![
                n(PackageBodyElement::AliasDef(n(AliasDef {
                    identification: id("Torque"),
                    target: "ISQ::TorqueValue".to_string(),
                    body: AliasBody::Brace,
                }))),
                n(PackageBodyElement::AttributeDef(n(AttributeDef {
                    name: "FuelCmd".to_string(),
                    typing: None,
                    body: AttributeBody::Semicolon,
                }))),
                n(PackageBodyElement::AttributeDef(n(AttributeDef {
                    name: "EngineStart".to_string(),
                    typing: None,
                    body: AttributeBody::Semicolon,
                }))),
                n(PackageBodyElement::AttributeDef(n(AttributeDef {
                    name: "EngineOff".to_string(),
                    typing: None,
                    body: AttributeBody::Semicolon,
                }))),
                n(PackageBodyElement::ActionDef(n(ActionDef {
                    identification: id("Generate Torque"),
                    body: ActionDefBody::Brace {
                        elements: vec![
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::In,
                                name: "fuelCmd".to_string(),
                                type_name: "FuelCmd".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "engineTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                        ],
                    },
                }))),
                n(PackageBodyElement::ActionDef(n(ActionDef {
                    identification: id("Amplify Torque"),
                    body: ActionDefBody::Brace {
                        elements: vec![
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::In,
                                name: "engineTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "transmissionTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                        ],
                    },
                }))),
                n(PackageBodyElement::ActionDef(n(ActionDef {
                    identification: id("Transfer Torque"),
                    body: ActionDefBody::Brace {
                        elements: vec![
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::In,
                                name: "transmissionTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "driveshaftTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                        ],
                    },
                }))),
                n(PackageBodyElement::ActionDef(n(ActionDef {
                    identification: id("Distribute Torque"),
                    body: ActionDefBody::Brace {
                        elements: vec![
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::In,
                                name: "driveShaftTorque".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "wheelTorque1".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "wheelTorque2".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                        ],
                    },
                }))),
                n(PackageBodyElement::ActionDef(n(ActionDef {
                    identification: id("Provide Power"),
                    body: ActionDefBody::Brace {
                        elements: vec![
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::In,
                                name: "fuelCmd".to_string(),
                                type_name: "FuelCmd".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "wheelTorque1".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                            n(ActionDefBodyElement::InOutDecl(n(InOutDecl {
                                direction: InOut::Out,
                                name: "wheelTorque2".to_string(),
                                type_name: "Torque".to_string(),
                            }))),
                        ],
                    },
                }))),
            ],
        },
    }
}

fn usages_package() -> Package {
    Package {
        identification: id("Usages"),
        body: PackageBody::Brace {
            elements: vec![n(PackageBodyElement::ActionUsage(n(provide_power_action())))],
        },
    }
}

fn provide_power_action() -> ActionUsage {
    ActionUsage {
        name: "provide power".to_string(),
        type_name: "Provide Power".to_string(),
        accept: None,
        body: ActionUsageBody::Brace {
            elements: vec![
                n(ActionUsageBodyElement::InOutDecl(n(InOutDecl {
                    direction: InOut::In,
                    name: "fuelCmd".to_string(),
                    type_name: "FuelCmd".to_string(),
                }))),
                n(ActionUsageBodyElement::InOutDecl(n(InOutDecl {
                    direction: InOut::Out,
                    name: "wheelTorque1".to_string(),
                    type_name: "Torque".to_string(),
                }))),
                n(ActionUsageBodyElement::InOutDecl(n(InOutDecl {
                    direction: InOut::Out,
                    name: "wheelTorque2".to_string(),
                    type_name: "Torque".to_string(),
                }))),
                n(ActionUsageBodyElement::Bind(n(Bind {
                    left: expr_path("generate torque.fuelCmd"),
                    right: expr_path("fuelCmd"),
                    body: Some(ConnectBody::Brace),
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "generate torque".to_string(),
                    type_name: "Generate Torque".to_string(),
                    accept: None,
                    body: ActionUsageBody::Brace { elements: vec![] },
                })))),
                n(ActionUsageBodyElement::Flow(n(Flow {
                    from: expr_path("generate torque.engineTorque"),
                    to: expr_path("amplify torque.engineTorque"),
                    body: ConnectBody::Brace,
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "amplify torque".to_string(),
                    type_name: "Amplify Torque".to_string(),
                    accept: None,
                    body: ActionUsageBody::Semicolon,
                })))),
                n(ActionUsageBodyElement::Flow(n(Flow {
                    from: expr_path("amplify torque.transmissionTorque"),
                    to: expr_path("transfer torque.transmissionTorque"),
                    body: ConnectBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "transfer torque".to_string(),
                    type_name: "Transfer Torque".to_string(),
                    accept: None,
                    body: ActionUsageBody::Semicolon,
                })))),
                n(ActionUsageBodyElement::Flow(n(Flow {
                    from: expr_path("transfer torque.driveshaftTorque"),
                    to: expr_path("distribute torque.driveShaftTorque"),
                    body: ConnectBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "distribute torque".to_string(),
                    type_name: "Distribute Torque".to_string(),
                    accept: None,
                    body: ActionUsageBody::Semicolon,
                })))),
                n(ActionUsageBodyElement::Bind(n(Bind {
                    left: expr_path("wheelTorque1"),
                    right: expr_path("distribute torque.wheelTorque1"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(ActionUsageBodyElement::Bind(n(Bind {
                    left: expr_path("wheelTorque2"),
                    right: expr_path("distribute torque.wheelTorque2"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("start".to_string())),
                    then: n(Expression::FeatureRef("continue".to_string())),
                    body: FirstMergeBody::Brace,
                }))),
                n(ActionUsageBodyElement::MergeStmt(n(MergeStmt {
                    merge: n(Expression::FeatureRef("continue".to_string())),
                    body: FirstMergeBody::Brace,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("continue".to_string())),
                    then: n(Expression::FeatureRef("engineStarted".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "engineStarted".to_string(),
                    type_name: "EngineStart".to_string(),
                    accept: Some(("engineStart".to_string(), "EngineStart".to_string())),
                    body: ActionUsageBody::Brace { elements: vec![] },
                })))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStarted".to_string())),
                    then: n(Expression::FeatureRef("engineStopped".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::ActionUsage(Box::new(n(ActionUsage {
                    name: "engineStopped".to_string(),
                    type_name: "EngineOff".to_string(),
                    accept: Some(("engineOff".to_string(), "EngineOff".to_string())),
                    body: ActionUsageBody::Semicolon,
                })))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStopped".to_string())),
                    then: n(Expression::FeatureRef("continue".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStarted".to_string())),
                    then: n(Expression::FeatureRef("generate torque".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStarted".to_string())),
                    then: n(Expression::FeatureRef("amplify torque".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStarted".to_string())),
                    then: n(Expression::FeatureRef("transfer torque".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("engineStarted".to_string())),
                    then: n(Expression::FeatureRef("distribute torque".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("generate torque".to_string())),
                    then: n(Expression::FeatureRef("engineStopped".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("amplify torque".to_string())),
                    then: n(Expression::FeatureRef("engineStopped".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("transfer torque".to_string())),
                    then: n(Expression::FeatureRef("engineStopped".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
                n(ActionUsageBodyElement::FirstStmt(n(FirstStmt {
                    first: n(Expression::FeatureRef("distribute torque".to_string())),
                    then: n(Expression::FeatureRef("engineStopped".to_string())),
                    body: FirstMergeBody::Semicolon,
                }))),
            ],
        },
    }
}

fn validation_fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sysml-v2-release")
        .join("sysml")
        .join("src")
        .join("validation")
        .join(relative)
}

#[test]
fn test_parse_3a_function_based_behavior() {
    super::init_log();
    let path = validation_fixture_path("03-Function-based Behavior")
        .join("3a-Function-based Behavior-1.sysml");
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    log::debug!("input len: {} bytes", input.len());
    let result = parse(&input)
        .expect("parse should succeed for 3a-Function-based Behavior-1.sysml");
    let expected = expected_ast();
    assert_eq!(
        result, expected,
        "parsed AST should match expected for 3a-Function-based Behavior-1.sysml"
    );
}
