//! Parser test for `02-Parts Interconnection/2a-Parts Interconnection.sysml`.

use sysml_parser::ast::{
    Bind, Connect, ConnectBody, ConnectStmt, EndDecl, Expression, Identification, Import,
    InterfaceDef, InterfaceDefBody, InterfaceDefBodyElement, InterfaceUsage,
    InterfaceUsageBodyElement, Node, Package, PackageBody, PackageBodyElement, PartDef,
    PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement, PortBody,
    PortDef, PortDefBody, PortDefBodyElement, PortUsage, RefBody, RefDecl, RootElement,
    RootNamespace, Span, Visibility,
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

/// Path expression from dot-separated path (e.g. "engine.fuelCmdPort").
fn expr_path(path: &str) -> Node<Expression> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut expr = Expression::FeatureRef(segments[0].to_string());
    for seg in segments.iter().skip(1) {
        expr = Expression::MemberAccess(Box::new(n(expr)), (*seg).to_string());
    }
    n(expr)
}

/// Index expression base#(n).
fn expr_index(base: &str, index_val: i64) -> Node<Expression> {
    n(Expression::Index {
        base: Box::new(n(Expression::FeatureRef(base.to_string()))),
        index: Box::new(n(Expression::LiteralInteger(index_val))),
    })
}

/// Expected AST for `2a-Parts Interconnection.sysml`.
fn expected_ast() -> RootNamespace {
    RootNamespace {
        elements: vec![n(RootElement::Package(n(Package {
            identification: id("2a-Parts Interconnection"),
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
                n(port_def_semicolon("FuelCmdPort")),
                n(port_def_semicolon("DrivePwrPort")),
                n(port_def_semicolon("ClutchPort")),
                n(port_def_semicolon("ShaftPort_a")),
                n(port_def_semicolon("ShaftPort_b")),
                n(port_def_semicolon("ShaftPort_c")),
                n(port_def_semicolon("ShaftPort_d")),
                n(port_def_semicolon("DiffPort")),
                n(port_def_semicolon("AxlePort")),
                n(port_def_semicolon("AxleToWheelPort")),
                n(port_def_semicolon("WheelToAxlePort")),
                n(port_def_semicolon("WheelToRoadPort")),
                n(port_def_vehicle_to_road()),
                n(part_def_vehicle_a()),
                n(PackageBodyElement::PartDef(n(PartDef {
                    definition_prefix: None,
                    identification: id("AxleAssembly"),
                    specializes: None,
                    specializes_span: None,
                    body: PartDefBody::Semicolon,
                }))),
                n(part_def_rear_axle_assembly()),
                n(PackageBodyElement::PartDef(n(PartDef {
                    definition_prefix: None,
                    identification: id("Axle"),
                    specializes: None,
                    specializes_span: None,
                    body: PartDefBody::Semicolon,
                }))),
                n(PackageBodyElement::PartDef(n(PartDef {
                    definition_prefix: None,
                    identification: id("RearAxle"),
                    specializes: Some("Axle".to_string()),
                    specializes_span: None,
                    body: PartDefBody::Semicolon,
                }))),
                n(part_def_half_axle()),
                n(part_def_engine()),
                n(part_def_transmission()),
                n(part_def_driveshaft()),
                n(PackageBodyElement::PartDef(n(PartDef {
                    definition_prefix: None,
                    identification: id("Differential"),
                    specializes: None,
                    specializes_span: None,
                    body: PartDefBody::Brace { elements: vec![] },
                }))),
                n(PackageBodyElement::PartDef(n(PartDef {
                    definition_prefix: None,
                    identification: id("Wheel"),
                    specializes: None,
                    specializes_span: None,
                    body: PartDefBody::Semicolon,
                }))),
                n(interface_def_engine_to_transmission()),
                n(interface_def_driveshaft()),
            ],
        },
    }
}

fn port_def_semicolon(name: &str) -> PackageBodyElement {
    PackageBodyElement::PortDef(n(PortDef {
        identification: id(name),
        specializes: None,
        body: PortDefBody::Semicolon,
    }))
}

fn port_def_vehicle_to_road() -> PackageBodyElement {
    PackageBodyElement::PortDef(n(PortDef {
        identification: id("VehicleToRoadPort"),
        specializes: None,
        body: PortDefBody::Brace {
            elements: vec![n(PortDefBodyElement::PortUsage(n(PortUsage {
                name: "wheelToRoadPort".to_string(),
                type_name: Some("WheelToRoadPort".to_string()),
                multiplicity: Some("[2]".to_string()),
                subsets: None,
                redefines: None,
                body: PortBody::Semicolon,
                name_span: None,
                type_ref_span: None,
            })))],
        },
    }))
}

fn part_def_vehicle_a() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("VehicleA"),
        specializes: None,
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "fuelCmdPort".to_string(),
                    type_name: Some("FuelCmdPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "vehicleToRoadPort".to_string(),
                    type_name: Some("VehicleToRoadPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn part_def_rear_axle_assembly() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("RearAxleAssembly"),
        specializes: Some("AxleAssembly".to_string()),
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![n(PartDefBodyElement::PortUsage(n(PortUsage {
                name: "shaftPort_d".to_string(),
                type_name: Some("ShaftPort_d".to_string()),
                multiplicity: None,
                subsets: None,
                redefines: None,
                body: PortBody::Semicolon,
                name_span: None,
                type_ref_span: None,
            })))],
        },
    }))
}

fn part_def_half_axle() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("HalfAxle"),
        specializes: None,
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "axleToDiffPort".to_string(),
                    type_name: Some("AxlePort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "axleToWheelPort".to_string(),
                    type_name: Some("AxleToWheelPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn part_def_engine() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("Engine"),
        specializes: None,
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "fuelCmdPort".to_string(),
                    type_name: Some("FuelCmdPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "drivePwrPort".to_string(),
                    type_name: Some("DrivePwrPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn part_def_transmission() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("Transmission"),
        specializes: None,
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "clutchPort".to_string(),
                    type_name: Some("ClutchPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_a".to_string(),
                    type_name: Some("ShaftPort_a".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn part_def_driveshaft() -> PackageBodyElement {
    PackageBodyElement::PartDef(n(PartDef {
        definition_prefix: None,
        identification: id("Driveshaft"),
        specializes: None,
        specializes_span: None,
        body: PartDefBody::Brace {
            elements: vec![
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_b".to_string(),
                    type_name: Some("ShaftPort_b".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartDefBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_c".to_string(),
                    type_name: Some("ShaftPort_c".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn interface_def_engine_to_transmission() -> PackageBodyElement {
    PackageBodyElement::InterfaceDef(n(InterfaceDef {
        identification: id("EngineToTransmissionInterface"),
        body: InterfaceDefBody::Brace {
            elements: vec![
                n(InterfaceDefBodyElement::EndDecl(n(EndDecl {
                    name: "drivePwrPort".to_string(),
                    type_name: "DrivePwrPort".to_string(),
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(InterfaceDefBodyElement::EndDecl(n(EndDecl {
                    name: "clutchPort".to_string(),
                    type_name: "ClutchPort".to_string(),
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }))
}

fn interface_def_driveshaft() -> PackageBodyElement {
    PackageBodyElement::InterfaceDef(n(InterfaceDef {
        identification: id("DriveshaftInterface"),
        body: InterfaceDefBody::Brace {
            elements: vec![
                n(InterfaceDefBodyElement::EndDecl(n(EndDecl {
                    name: "shaftPort_a".to_string(),
                    type_name: "ShaftPort_a".to_string(),
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(InterfaceDefBodyElement::EndDecl(n(EndDecl {
                    name: "shaftPort_d".to_string(),
                    type_name: "ShaftPort_d".to_string(),
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(InterfaceDefBodyElement::RefDecl(n(RefDecl {
                    name: "driveshaft".to_string(),
                    type_name: "Driveshaft".to_string(),
                    value: None,
                    body: RefBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(InterfaceDefBodyElement::ConnectStmt(n(ConnectStmt {
                    from: n(Expression::FeatureRef("shaftPort_a".to_string())),
                    to: expr_path("driveshaft.shaftPort_b"),
                    body: ConnectBody::Brace,
                }))),
                n(InterfaceDefBodyElement::ConnectStmt(n(ConnectStmt {
                    from: expr_path("driveshaft.shaftPort_c"),
                    to: n(Expression::FeatureRef("shaftPort_d".to_string())),
                    body: ConnectBody::Semicolon,
                }))),
            ],
        },
    }))
}

fn usages_package() -> Package {
    Package {
        identification: id("Usages"),
        body: PackageBody::Brace {
            elements: vec![n(PackageBodyElement::PartUsage(n(part_vehicle1_c1())))],
        },
    }
}

fn part_vehicle1_c1() -> PartUsage {
    PartUsage {
        name: "vehicle1_c1".to_string(),
        type_name: "VehicleA".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: None,
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::Bind(n(Bind {
                    left: expr_path("fuelCmdPort"),
                    right: expr_path("engine.fuelCmdPort"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "engine".to_string(),
                    type_name: "Engine".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::InterfaceUsage(n(
                    InterfaceUsage::TypedConnect {
                        interface_type: Some("EngineToTransmissionInterface".to_string()),
                        from: expr_path("engine.drivePwrPort"),
                        to: expr_path("transmission.clutchPort"),
                        body: ConnectBody::Brace,
                        body_elements: vec![],
                    },
                ))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "transmission".to_string(),
                    type_name: "Transmission".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "driveshaft".to_string(),
                    type_name: "Driveshaft".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Brace { elements: vec![] },
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::InterfaceUsage(n(
                    InterfaceUsage::TypedConnect {
                        interface_type: Some("DriveshaftInterface".to_string()),
                        from: expr_path("transmission.shaftPort_a"),
                        to: expr_path("rearAxleAssembly.shaftPort_d"),
                        body: ConnectBody::Brace,
                        body_elements: vec![n(InterfaceUsageBodyElement::RefRedef {
                            name: "driveshaft".to_string(),
                            value: expr_path("vehicle1_c1.driveshaft"),
                            body: RefBody::Brace,
                        })],
                    },
                ))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_rear_axle_assembly(),
                )))),
                n(PartUsageBodyElement::Bind(n(Bind {
                    left: expr_path("rearAxleAssembly.leftWheel.wheelToRoadPort"),
                    right: expr_path("vehicleToRoadPort.leftWheelToRoadPort"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(PartUsageBodyElement::Bind(n(Bind {
                    left: expr_path("rearAxleAssembly.rightWheel.wheelToRoadPort"),
                    right: expr_path("vehicleToRoadPort.rightWheelToRoadPort"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "vehicleToRoadPort".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: Some("VehicleA::vehicleToRoadPort".to_string()),
                    body: PortBody::BraceWithPorts {
                        elements: vec![
                            n(PortUsage {
                                name: "leftWheelToRoadPort".to_string(),
                                type_name: None,
                                multiplicity: None,
                                subsets: Some((
                                    "wheelToRoadPort".to_string(),
                                    Some(expr_index("wheelToRoadPort", 1)),
                                )),
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }),
                            n(PortUsage {
                                name: "rightWheelToRoadPort".to_string(),
                                type_name: None,
                                multiplicity: None,
                                subsets: Some((
                                    "wheelToRoadPort".to_string(),
                                    Some(expr_index("wheelToRoadPort", 2)),
                                )),
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }),
                        ],
                    },
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }
}

fn part_rear_axle_assembly() -> PartUsage {
    PartUsage {
        name: "rearAxleAssembly".to_string(),
        type_name: "RearAxleAssembly".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: None,
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::Bind(n(Bind {
                    left: expr_path("shaftPort_d"),
                    right: expr_path("differential.shaftPort_d"),
                    body: Some(ConnectBody::Semicolon),
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_differential(),
                )))),
                n(PartUsageBodyElement::InterfaceUsage(n(
                    InterfaceUsage::Connection {
                        from: expr_path("differential.leftDiffPort"),
                        to: expr_path("rearAxle.leftHalfAxle.axleToDiffPort"),
                        body_elements: vec![],
                    },
                ))),
                n(PartUsageBodyElement::InterfaceUsage(n(
                    InterfaceUsage::Connection {
                        from: expr_path("differential.rightDiffPort"),
                        to: expr_path("rearAxle.rightHalfAxle.axleToDiffPort"),
                        body_elements: vec![],
                    },
                ))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_rear_axle(),
                )))),
                n(PartUsageBodyElement::Connect(n(Connect {
                    from: expr_path("rearAxle.leftHalfAxle.axleToWheelPort"),
                    to: expr_path("leftWheel.wheelToAxlePort"),
                    body: ConnectBody::Semicolon,
                }))),
                n(PartUsageBodyElement::Connect(n(Connect {
                    from: expr_path("rearAxle.rightHalfAxle.axleToWheelPort"),
                    to: expr_path("rightWheel.wheelToAxlePort"),
                    body: ConnectBody::Semicolon,
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "rearWheel".to_string(),
                    type_name: "Wheel".to_string(),
                    multiplicity: Some("[2]".to_string()),
                    ordered: true,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "leftWheel".to_string(),
                    type_name: "".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: Some(("rearWheel".to_string(), Some(expr_index("rearWheel", 1)))),
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PortUsage(n(PortUsage {
                                name: "wheelToAxlePort".to_string(),
                                type_name: Some("WheelToAxlePort".to_string()),
                                multiplicity: None,
                                subsets: None,
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }))),
                            n(PartUsageBodyElement::PortUsage(n(PortUsage {
                                name: "wheelToRoadPort".to_string(),
                                type_name: Some("WheelToRoadPort".to_string()),
                                multiplicity: None,
                                subsets: None,
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }))),
                        ],
                    },
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "rightWheel".to_string(),
                    type_name: "".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: Some(("rearWheel".to_string(), Some(expr_index("rearWheel", 2)))),
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            n(PartUsageBodyElement::PortUsage(n(PortUsage {
                                name: "wheelToAxlePort".to_string(),
                                type_name: Some("WheelToAxlePort".to_string()),
                                multiplicity: None,
                                subsets: None,
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }))),
                            n(PartUsageBodyElement::PortUsage(n(PortUsage {
                                name: "wheelToRoadPort".to_string(),
                                type_name: Some("WheelToRoadPort".to_string()),
                                multiplicity: None,
                                subsets: None,
                                redefines: None,
                                body: PortBody::Semicolon,
                                name_span: None,
                                type_ref_span: None,
                            }))),
                        ],
                    },
                    name_span: None,
                    type_ref_span: None,
                })))),
            ],
        },
    }
}

fn part_differential() -> PartUsage {
    PartUsage {
        name: "differential".to_string(),
        type_name: "Differential".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: None,
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_d".to_string(),
                    type_name: Some("ShaftPort_d".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "leftDiffPort".to_string(),
                    type_name: Some("DiffPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "rightDiffPort".to_string(),
                    type_name: Some("DiffPort".to_string()),
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }
}

fn part_rear_axle() -> PartUsage {
    PartUsage {
        name: "rearAxle".to_string(),
        type_name: "RearAxle".to_string(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: None,
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "leftHalfAxle".to_string(),
                    type_name: "HalfAxle".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                })))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(PartUsage {
                    name: "rightHalfAxle".to_string(),
                    type_name: "HalfAxle".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    redefines: None,
                    value: None,
                    body: PartUsageBody::Semicolon,
                    name_span: None,
                    type_ref_span: None,
                })))),
            ],
        },
    }
}

/// Uses SYSML_V2_RELEASE_DIR when set (CI); otherwise sysml-v2-release in repo.
fn validation_fixture_path(relative: &str) -> std::path::PathBuf {
    let root = std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release")
        });
    root.join("sysml")
        .join("src")
        .join("validation")
        .join(relative)
}

#[test]
#[ignore = "requires SysML v2 release fixtures; run with: cargo test --test validation -- --include-ignored"]
fn test_parse_2a_parts_interconnection() {
    super::init_log();
    let path =
        validation_fixture_path("02-Parts Interconnection").join("2a-Parts Interconnection.sysml");
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    log::debug!("input len: {} bytes", input.len());
    let result = parse(&input).expect("parse should succeed for 2a-Parts Interconnection.sysml");
    let expected = expected_ast();
    super::assert_ast_eq(
        &result,
        &expected,
        "parsed AST should match expected for 2a-Parts Interconnection.sysml",
    );
}
