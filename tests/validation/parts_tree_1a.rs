//! Parser test for `01-Parts Tree/1a-Parts Tree.sysml`.

use std::path::Path;
use sysml_parser::ast::{
    AttributeBody, AttributeDef, AttributeUsage, Identification, Import, Package, PackageBody,
    PackageBodyElement, PartDef, PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody,
    PartUsageBodyElement, RootNamespace, Visibility,
};
use sysml_parser::parse;

fn id(name: &str) -> Identification {
    Identification {
        short_name: None,
        name: Some(name.to_string()),
    }
}

/// Expected AST for `1a-Parts Tree.sysml`: full structure with package, import, part def, part usage.
fn expected_ast() -> RootNamespace {
    RootNamespace {
        elements: vec![PackageBodyElement::Package(Package {
            identification: id("1a-Parts Tree"),
            body: PackageBody::Brace {
                elements: vec![
                    PackageBodyElement::Import(Import {
                        visibility: Some(Visibility::Private),
                        is_import_all: false,
                        target: "SI::kg".to_string(),
                    }),
                    PackageBodyElement::Package(Package {
                        identification: id("Definitions"),
                        body: PackageBody::Brace {
                            elements: vec![
                                PackageBodyElement::PartDef(PartDef {
                                    identification: id("Vehicle"),
                                    specializes: None,
                                    body: PartDefBody::Brace {
                                        elements: vec![PartDefBodyElement::AttributeDef(
                                            AttributeDef {
                                                name: "mass".to_string(),
                                                typing: Some("ISQ::mass".to_string()),
                                                body: AttributeBody::Brace,
                                            },
                                        )],
                                    },
                                }),
                                PackageBodyElement::PartDef(PartDef {
                                    identification: id("AxleAssembly"),
                                    specializes: None,
                                    body: PartDefBody::Semicolon,
                                }),
                                PackageBodyElement::PartDef(PartDef {
                                    identification: id("Axle"),
                                    specializes: None,
                                    body: PartDefBody::Brace {
                                        elements: vec![PartDefBodyElement::AttributeDef(
                                            AttributeDef {
                                                name: "mass".to_string(),
                                                typing: Some("ISQ::mass".to_string()),
                                                body: AttributeBody::Semicolon,
                                            },
                                        )],
                                    },
                                }),
                                PackageBodyElement::PartDef(PartDef {
                                    identification: id("FrontAxle"),
                                    specializes: Some("Axle".to_string()),
                                    body: PartDefBody::Brace {
                                        elements: vec![PartDefBodyElement::AttributeDef(
                                            AttributeDef {
                                                name: "steeringAngle".to_string(),
                                                typing: Some("ScalarValues::Real".to_string()),
                                                body: AttributeBody::Semicolon,
                                            },
                                        )],
                                    },
                                }),
                                PackageBodyElement::PartDef(PartDef {
                                    identification: id("Wheel"),
                                    specializes: None,
                                    body: PartDefBody::Semicolon,
                                }),
                            ],
                        },
                    }),
                    PackageBodyElement::Package(Package {
                        identification: id("Usages"),
                        body: PackageBody::Brace {
                            elements: vec![
                                PackageBodyElement::Import(Import {
                                    visibility: Some(Visibility::Private),
                                    is_import_all: true,
                                    target: "Definitions::*".to_string(),
                                }),
                                PackageBodyElement::PartUsage(part_vehicle1()),
                                PackageBodyElement::PartUsage(part_vehicle1_c1()),
                            ],
                        },
                    }),
                ],
            },
        })],
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
                PartUsageBodyElement::AttributeUsage(AttributeUsage {
                    name: "mass".to_string(),
                    redefines: Some("Vehicle::mass".to_string()),
                    value: Some("1750 [kg]".to_string()),
                    body: AttributeBody::Brace,
                }),
                PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                    name: "frontAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })),
                        ],
                    },
                })),
                PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                    name: "rearAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })),
                        ],
                    },
                })),
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
                PartUsageBodyElement::AttributeUsage(AttributeUsage {
                    name: "mass".to_string(),
                    redefines: Some("Vehicle::mass".to_string()),
                    value: Some("2000 [kg]".to_string()),
                    body: AttributeBody::Brace,
                }),
                PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                    name: "frontAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontAxle".to_string(),
                                type_name: "FrontAxle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Brace { elements: vec![] },
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontWheel_1".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "frontWheel".to_string(),
                                    Some("frontWheel#(1)".to_string()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "frontWheel_2".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "frontWheel".to_string(),
                                    Some("frontWheel#(2)".to_string()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })),
                        ],
                    },
                })),
                PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                    name: "rearAxleAssembly".to_string(),
                    type_name: "AxleAssembly".to_string(),
                    multiplicity: None,
                    ordered: false,
                    subsets: None,
                    body: PartUsageBody::Brace {
                        elements: vec![
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearAxle".to_string(),
                                type_name: "Axle".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearWheel".to_string(),
                                type_name: "Wheel".to_string(),
                                multiplicity: Some("[2]".to_string()),
                                ordered: true,
                                subsets: None,
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearWheel_1".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "rearWheel".to_string(),
                                    Some("rearWheel#(1)".to_string()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })),
                            PartUsageBodyElement::PartUsage(Box::new(PartUsage {
                                name: "rearWheel_2".to_string(),
                                type_name: "".to_string(),
                                multiplicity: None,
                                ordered: false,
                                subsets: Some((
                                    "rearWheel".to_string(),
                                    Some("rearWheel#(2)".to_string()),
                                )),
                                body: PartUsageBody::Semicolon,
                            })),
                        ],
                    },
                })),
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
    let path = validation_fixture_path("01-Parts Tree").join("1a-Parts Tree.sysml");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let result = parse(&input).expect("parse should succeed for 1a-Parts Tree.sysml");
    let expected = expected_ast();
    assert_eq!(
        result, expected,
        "parsed AST should match expected for 1a-Parts Tree.sysml"
    );
}
