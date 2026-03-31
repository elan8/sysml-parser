//! Parser test for `04-Functional Allocation/4a-Functional Allocation.sysml`.

use sysml_parser::ast::{
    DocComment, Expression, Identification, Import, InOut, Node, Package, PackageBody,
    PackageBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement, Perform, PerformBody,
    PerformBodyElement, PerformInOutBinding, PortBody, PortUsage, RootElement, RootNamespace, Span,
    Visibility,
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
            identification: id("4a-Functional Allocation"),
            body: PackageBody::Brace {
                elements: vec![
                    n(PackageBodyElement::Import(n(Import {
                        visibility: Some(Visibility::Private),
                        is_import_all: true,
                        target: "2a-Parts Interconnection::*".to_string(),
                        is_recursive: false,
                        filter_members: None,
                    }))),
                    n(PackageBodyElement::Import(n(Import {
                        visibility: Some(Visibility::Private),
                        is_import_all: true,
                        target: "3a-Function-based Behavior-1::*".to_string(),
                        is_recursive: false,
                        filter_members: None,
                    }))),
                    n(PackageBodyElement::Import(n(Import {
                        visibility: Some(Visibility::Private),
                        is_import_all: true,
                        target: "3a-Function-based Behavior-1::provide power::*".to_string(),
                        is_recursive: false,
                        filter_members: None,
                    }))),
                    n(PackageBodyElement::PartUsage(n(
                        vehicle1_c1_functional_allocation(),
                    ))),
                ],
            },
        })))],
    }
}

fn vehicle1_c1_functional_allocation() -> PartUsage {
    PartUsage {
        name: "vehicle1_c1_functional_allocation".to_string(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: Some(("vehicle1_c1".to_string(), None)),
        redefines: None,
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(port_fuel_cmd_port()))),
                n(PartUsageBodyElement::Perform(n(Perform {
                    action_name: "provide power".to_string(),
                    type_name: None,
                    body: PerformBody::Brace {
                        elements: vec![
                            n(PerformBodyElement::Doc(n(DocComment {
                                identification: None,
                                locale: None,
                                text: "\n\t\t * This allocates the action '3a-Function-based Behavior-1'::'provide power' as an enacted \n\t\t * performance of 'vehicle_c1_functional_allocation'.\n\t\t ".to_string(),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::In,
                                name: "fuelCmd".to_string(),
                                value: expr_path("fuelCmdPort.fuelCmd"),
                            }))),
                        ],
                    },
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(part_engine())))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(part_transmission())))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(part_driveshaft())))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(part_rear_axle_assembly())))),
            ],
        },
    }
}

fn port_fuel_cmd_port() -> PortUsage {
    PortUsage {
        name: "fuelCmdPort".to_string(),
        type_name: None,
        multiplicity: None,
        subsets: None,
        redefines: None,
        body: PortBody::Brace,
        name_span: None,
        type_ref_span: None,
    }
}

fn part_engine() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("engine".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "fuelCmdPort".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::Perform(n(Perform {
                    action_name: "provide power.generate torque".to_string(),
                    type_name: None,
                    body: PerformBody::Brace {
                        elements: vec![
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::In,
                                name: "fuelCmd".to_string(),
                                value: expr_path("fuelCmdPort.fuelCmd"),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::Out,
                                name: "engineTorque".to_string(),
                                value: expr_path("drivePwrPort.engineTorque"),
                            }))),
                        ],
                    },
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "drivePwrPort".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }
}

fn part_transmission() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("transmission".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "clutchPort".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::Perform(n(Perform {
                    action_name: "provide power.amplify torque".to_string(),
                    type_name: None,
                    body: PerformBody::Brace {
                        elements: vec![
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::In,
                                name: "engineTorque".to_string(),
                                value: expr_path("clutchPort.engineTorque"),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::Out,
                                name: "transmissionTorque".to_string(),
                                value: expr_path("shaftPort_a.transmissionTorque"),
                            }))),
                        ],
                    },
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_a".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }
}

fn part_driveshaft() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("driveshaft".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_b".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::Perform(n(Perform {
                    action_name: "provide power.transfer torque".to_string(),
                    type_name: None,
                    body: PerformBody::Brace {
                        elements: vec![
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::In,
                                name: "transmissionTorque".to_string(),
                                value: expr_path("shaftPort_b.transmissionTorque"),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::Out,
                                name: "driveshaftTorque".to_string(),
                                value: expr_path("shaftPort_c.driveshaftTorque"),
                            }))),
                        ],
                    },
                }))),
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_c".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
            ],
        },
    }
}

fn part_rear_axle_assembly() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("rearAxleAssembly".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PortUsage(n(PortUsage {
                    name: "shaftPort_d".to_string(),
                    type_name: None,
                    multiplicity: None,
                    subsets: None,
                    redefines: None,
                    body: PortBody::Brace,
                    name_span: None,
                    type_ref_span: None,
                }))),
                n(PartUsageBodyElement::Perform(n(Perform {
                    action_name: "provide power.distribute torque".to_string(),
                    type_name: None,
                    body: PerformBody::Brace {
                        elements: vec![
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::In,
                                name: "driveshaftTorque".to_string(),
                                value: expr_path("shaftPort_d.driveshaftTorque"),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::Out,
                                name: "wheelTorque1".to_string(),
                                value: expr_path(
                                    "rearAxle.leftHalfAxle.axleToWheelPort.wheelTorque",
                                ),
                            }))),
                            n(PerformBodyElement::InOut(n(PerformInOutBinding {
                                direction: InOut::Out,
                                name: "wheelTorque2".to_string(),
                                value: expr_path(
                                    "rearAxle.rightHalfAxle.axleToWheelPort.wheelTorque",
                                ),
                            }))),
                        ],
                    },
                }))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_rear_axle(),
                )))),
            ],
        },
    }
}

fn part_rear_axle() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("rearAxle".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_left_half_axle(),
                )))),
                n(PartUsageBodyElement::PartUsage(Box::new(n(
                    part_right_half_axle(),
                )))),
            ],
        },
    }
}

fn part_left_half_axle() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("leftHalfAxle".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![n(PartUsageBodyElement::PortUsage(n(PortUsage {
                name: "axleToWheelPort".to_string(),
                type_name: None,
                multiplicity: None,
                subsets: None,
                redefines: None,
                body: PortBody::Brace,
                name_span: None,
                type_ref_span: None,
            })))],
        },
    }
}

fn part_right_half_axle() -> PartUsage {
    PartUsage {
        name: String::new(),
        type_name: String::new(),
        multiplicity: None,
        ordered: false,
        subsets: None,
        redefines: Some("rightHalfAxle".to_string()),
        value: None,
        name_span: None,
        type_ref_span: None,
        body: PartUsageBody::Brace {
            elements: vec![n(PartUsageBodyElement::PortUsage(n(PortUsage {
                name: "axleToWheelPort".to_string(),
                type_name: None,
                multiplicity: None,
                subsets: None,
                redefines: None,
                body: PortBody::Brace,
                name_span: None,
                type_ref_span: None,
            })))],
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
fn test_parse_4a_functional_allocation() {
    super::init_log();
    let path =
        validation_fixture_path("04-Functional Allocation").join("4a-Functional Allocation.sysml");
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    // Normalize line endings so parsing is consistent (fixture may have CRLF on Windows).
    let input = input.replace("\r\n", "\n").replace('\r', "\n");
    log::debug!("input len: {} bytes", input.len());
    let result = parse(&input);
    let parsed = match &result {
        Ok(ast) => ast,
        Err(e) => panic!(
            "parse should succeed for 4a-Functional Allocation.sysml: {:?}",
            e
        ),
    };
    let expected = expected_ast();
    super::assert_ast_eq(
        parsed,
        &expected,
        "parsed AST should match expected for 4a-Functional Allocation.sysml",
    );
}
