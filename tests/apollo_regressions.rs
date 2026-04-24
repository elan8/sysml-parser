use sysml_v2_parser::ast::{
    ActionDefBody, ConnectionDefBody, ConnectionDefBodyElement, ConstraintDefBody,
    ConstraintDefBodyElement, Expression, OccurrenceBodyElement, OccurrenceUsageBody, PackageBody,
    PackageBodyElement, PartDefBody, PartDefBodyElement, PartUsageBody, PartUsageBodyElement,
    RequirementDefBody, RequirementDefBodyElement, RootElement, StateDefBody, StateDefBodyElement,
};
use sysml_v2_parser::{parse, parse_with_diagnostics};

fn package_elements(input: &str) -> Vec<sysml_v2_parser::Node<PackageBodyElement>> {
    let root = parse(input).expect("input should parse");
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        other => panic!("expected package, got {other:?}"),
    };
    match &pkg.body {
        PackageBody::Brace { elements } => elements.clone(),
        _ => panic!("expected brace package body"),
    }
}

#[test]
fn individual_part_definition_and_usage_parse_as_parts() {
    let input = "package P {\nindividual part def 'Neil Armstrong' :> Astronaut { }\nindividual part crewMember : Astronaut { }\n}";
    let elements = package_elements(input);

    match &elements[0].value {
        PackageBodyElement::PartDef(def) => {
            assert!(def.value.is_individual);
            assert_eq!(
                def.value.identification.name.as_deref(),
                Some("Neil Armstrong")
            );
        }
        other => panic!("expected individual part def, got {other:?}"),
    }

    match &elements[1].value {
        PackageBodyElement::PartUsage(usage) => {
            assert!(usage.value.is_individual);
            assert_eq!(usage.value.name, "crewMember");
        }
        other => panic!("expected individual part usage, got {other:?}"),
    }
}

#[test]
fn requirement_usage_supports_trailing_subsets_after_body() {
    let input = "package P {\npart def Mission {\nrequirement goals[1..*] : Goal;\n}\npart def ApolloMission :> Mission {\nrequirement goToMoon : Goal {\ndoc /* Perform a crewed lunar landing and return to Earth. */\n} :> goals;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let part = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("ApolloMission") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("ApolloMission part def should be present");
    let PartDefBody::Brace { elements } = &part.body else {
        panic!("expected part body");
    };
    let req = elements.iter().find_map(|e| match &e.value {
        PartDefBodyElement::RequirementUsage(req) => Some(&req.value),
        _ => None,
    });
    let req = req.expect("requirement usage should parse in part body");
    assert_eq!(req.subsets.as_deref(), Some("goals"));
}

#[test]
fn exhibit_state_body_supports_unnamed_and_accepting_transitions() {
    let input = "package P {\npart def Mission {\nexhibit state phases {\nstate initial : Initial;\nstate launch : Launch;\ntransition first initial then launch;\ntransition first launch accept LaunchDone then initial {\ndoc /* Example transition body. */\n}\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("Mission") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("expected Mission part def");
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let exhibit = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::ExhibitState(exhibit) => Some(&exhibit.value),
            _ => None,
        })
        .expect("exhibit state should be present");
    let StateDefBody::Brace { elements } = &exhibit.body else {
        panic!("expected exhibit state body");
    };
    let transitions: Vec<_> = elements
        .iter()
        .filter_map(|e| match &e.value {
            StateDefBodyElement::Transition(t) => Some(&t.value),
            _ => None,
        })
        .collect();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].name, None);
    assert_eq!(transitions[1].name, None);
}

#[test]
fn timeslice_and_snapshot_parse_inside_part_and_occurrence_bodies() {
    let input = "package P {\nindividual part def MissionIndividual :> Mission;\nindividual part mission : MissionIndividual {\ntimeslice liftoff {\nsnapshot atT0 {\nattribute missionTime = 0;\n}\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let usage = match &elements[1].value {
        PackageBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected part usage"),
    };
    let PartUsageBody::Brace { elements } = &usage.body else {
        panic!("expected part usage body");
    };
    let timeslice = elements
        .iter()
        .find_map(|e| match &e.value {
            PartUsageBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .expect("timeslice should parse in part body");
    assert_eq!(timeslice.portion_kind.as_deref(), Some("timeslice"));
    let OccurrenceUsageBody::Brace { elements } = &timeslice.body else {
        panic!("expected timeslice body");
    };
    let snapshot = elements
        .iter()
        .find_map(|e| match &e.value {
            OccurrenceBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .expect("snapshot should parse in timeslice body");
    assert_eq!(snapshot.portion_kind.as_deref(), Some("snapshot"));
}

#[test]
fn then_timeslice_and_specialized_snapshot_parse_inside_individual_part() {
    let input = "package P {\nindividual part def MissionIndividual :> Mission;\nindividual part mission : MissionIndividual {\ntimeslice ingress {\nassert constraint { ready }\nsnapshot atIngress :> system : MissionSystem {\nattribute missionTime = 0;\n}\n}\nthen timeslice liftoff {\nsnapshot atT0 :> system : MissionSystem {\nattribute missionTime = 1;\n}\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let usage = match &elements[1].value {
        PackageBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected part usage"),
    };
    let PartUsageBody::Brace { elements } = &usage.body else {
        panic!("expected part usage body");
    };
    let occurrences: Vec<_> = elements
        .iter()
        .filter_map(|e| match &e.value {
            PartUsageBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .collect();
    assert_eq!(occurrences.len(), 2);
    assert_eq!(occurrences[0].portion_kind.as_deref(), Some("timeslice"));
    assert_eq!(occurrences[0].subsets.as_deref(), None);
    assert_eq!(occurrences[1].portion_kind.as_deref(), Some("timeslice"));

    let OccurrenceUsageBody::Brace {
        elements: ingress_elements,
    } = &occurrences[0].body
    else {
        panic!("expected ingress timeslice body");
    };
    assert!(
        ingress_elements.iter().all(
            |e| !matches!(e.value, OccurrenceBodyElement::Other(ref text) if text == "assert constraint")
        ),
        "assert constraint should not degrade to OccurrenceBodyElement::Other"
    );
    let assert_constraint = ingress_elements
        .iter()
        .find_map(|e| match &e.value {
            OccurrenceBodyElement::AssertConstraint(member) => Some(&member.value),
            _ => None,
        })
        .expect("assert constraint should parse as a structured occurrence member");
    let ConstraintDefBody::Brace {
        elements: assert_elements,
    } = &assert_constraint.body
    else {
        panic!("expected assert constraint body");
    };
    assert!(
        assert_elements.iter().any(|e| {
            matches!(
                &e.value,
                ConstraintDefBodyElement::Expression(expr)
                    if matches!(&expr.value, Expression::FeatureRef(name) if name == "ready")
            )
        }),
        "assert constraint body should preserve the `ready` expression"
    );

    let OccurrenceUsageBody::Brace { elements } = &occurrences[1].body else {
        panic!("expected timeslice body");
    };
    let snapshot = elements
        .iter()
        .find_map(|e| match &e.value {
            OccurrenceBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .expect("snapshot should parse in then timeslice body");
    assert_eq!(snapshot.subsets.as_deref(), Some("system"));
    assert_eq!(snapshot.type_name.as_deref(), Some("MissionSystem"));
}

#[test]
fn anonymous_individual_parts_and_body_trailing_subsets_parse() {
    let input = "package P {\npart def Mission {\npart crew;\n}\npart apolloProgram {\npart apollo1 : Mission {\nindividual part : 'Gus Grissom' :> crew;\nindividual part : 'Ed White' :> crew;\n} :> missions;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let program = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartUsage(usage) if usage.value.name == "apolloProgram" => {
                Some(&usage.value)
            }
            _ => None,
        })
        .expect("expected program part usage");
    let PartUsageBody::Brace { elements } = &program.body else {
        panic!("expected program body");
    };
    let apollo1 = match &elements[0].value {
        PartUsageBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected nested part usage"),
    };
    assert_eq!(
        apollo1.subsets.as_ref().map(|(name, _)| name.as_str()),
        Some("missions")
    );

    let PartUsageBody::Brace { elements } = &apollo1.body else {
        panic!("expected nested mission body");
    };
    let crew_members: Vec<_> = elements
        .iter()
        .filter_map(|e| match &e.value {
            PartUsageBodyElement::PartUsage(usage) if usage.value.is_individual => {
                Some(&usage.value)
            }
            _ => None,
        })
        .collect();
    assert_eq!(crew_members.len(), 2);
    assert_eq!(crew_members[0].name, "");
    assert_eq!(crew_members[0].type_name, "Gus Grissom");
    assert_eq!(
        crew_members[0]
            .subsets
            .as_ref()
            .map(|(name, _)| name.as_str()),
        Some("crew")
    );
}

#[test]
fn exhibit_state_supports_trailing_redefinition_after_body() {
    let input = "package P {\npart def Mission {\nexhibit state phases {\nstate initial : Initial;\ntransition first initial then initial;\n} :>> missionPhases;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("Mission") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("expected Mission part def");
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let exhibit = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::ExhibitState(exhibit) => Some(&exhibit.value),
            _ => None,
        })
        .expect("exhibit state should be present");
    assert_eq!(exhibit.redefines.as_deref(), Some("missionPhases"));
}

#[test]
fn exhibit_state_body_accepts_requirement_usage_members() {
    let input = "package P {\nrequirement def Goal;\nstate def MissionPhase;\npart def Mission {\nrequirement goals[1..*] : Goal;\nexhibit state phases : MissionPhase {\nrequirement goToMoon : Goal {\ndoc /* Example */\n} :> goals;\nstate launch;\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("Mission") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("expected Mission part def");
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let exhibit = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::ExhibitState(exhibit) => Some(&exhibit.value),
            _ => None,
        })
        .expect("exhibit state should be present");
    let StateDefBody::Brace { elements } = &exhibit.body else {
        panic!("expected exhibit state body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, StateDefBodyElement::RequirementUsage(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, StateDefBodyElement::StateUsage(_))));
}

#[test]
fn part_usage_accepts_multiplicity_before_type() {
    let input = "package P {\npart def System {\npart spaceSuits[2] : ExtravehicularMobilityUnit :> constituentSystems;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let system = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &system.body else {
        panic!("expected part body");
    };
    let suit = match &elements[0].value {
        PartDefBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected part usage"),
    };
    assert_eq!(suit.name, "spaceSuits");
    assert_eq!(suit.multiplicity.as_deref(), Some("[2]"));
    assert_eq!(suit.type_name, "ExtravehicularMobilityUnit");
    assert_eq!(
        suit.subsets.as_ref().map(|(name, _)| name.as_str()),
        Some("constituentSystems")
    );
}

#[test]
fn rationale_and_refinement_annotations_stay_localized() {
    let input = "package P {\naction def PerformCrewIngress {\nout isCrewAboard: Boolean;\n@Rationale { }\n#refinement dependency PerformCrewIngress to OperationsPackage::TransferCrewToVehicle;\n}\nrequirement def R {\n@Rationale { }\n#refinement dependency 'HLR-R001' to CapabilitiesPackage::DeepSpaceHabitationAndLifeSupport;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "annotations should parse without recovery cascades: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    match &elements[0].value {
        PackageBodyElement::ActionDef(action) => {
            let ActionDefBody::Brace { elements } = &action.value.body else {
                panic!("expected action body");
            };
            assert!(elements.iter().any(|e| matches!(
                e.value,
                sysml_v2_parser::ActionDefBodyElement::Annotation(_)
            )));
        }
        _ => panic!("expected action def"),
    }
    match &elements[1].value {
        PackageBodyElement::RequirementDef(req) => {
            let RequirementDefBody::Brace { elements } = &req.value.body else {
                panic!("expected requirement body");
            };
            assert!(elements
                .iter()
                .any(|e| matches!(e.value, RequirementDefBodyElement::Annotation(_))));
        }
        _ => panic!("expected requirement def"),
    }
}

#[test]
fn quoted_requirement_identifier_parses() {
    let input = "package P {\nrequirement def <'HLR-R001'> CrewReturnSafetyRequirement { }\n}";
    let elements = package_elements(input);
    match &elements[0].value {
        PackageBodyElement::RequirementDef(req) => {
            assert_eq!(
                req.value.identification.short_name.as_deref(),
                Some("HLR-R001")
            );
            assert_eq!(
                req.value.identification.name.as_deref(),
                Some("CrewReturnSafetyRequirement")
            );
        }
        other => panic!("expected requirement def, got {other:?}"),
    }
}

#[test]
fn mission_capability_connections_with_trailing_subsets_parse() {
    let input = "package P {\npart def Mission {\nconnection : CapabilityToGoalDerivation {\nend capa ::> toolDevelopment;\nend goal ::> deploy;\n} :> capabilityToGoals;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let connection = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::Connection(connection) => Some(&connection.value),
            _ => None,
        })
        .expect("expected structured connection member in part body");
    assert_eq!(
        connection.type_name.as_deref(),
        Some("CapabilityToGoalDerivation")
    );
    assert_eq!(connection.subsets.as_deref(), Some("capabilityToGoals"));
    let ConnectionDefBody::Brace { elements } = &connection.body else {
        panic!("expected connection body");
    };
    assert_eq!(
        elements
            .iter()
            .filter(|el| matches!(el.value, ConnectionDefBodyElement::EndDecl(_)))
            .count(),
        2
    );
}

#[test]
fn part_definition_comment_members_parse_structurally() {
    let input =
        "package P {\npart def Mission {\ncomment source /* https://example.test/source */\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let comment = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::Comment(comment) => Some(&comment.value),
            _ => None,
        })
        .expect("expected structured comment member in part body");
    assert_eq!(
        comment
            .identification
            .as_ref()
            .and_then(|id| id.name.as_deref()),
        Some("source")
    );
    assert!(comment.text.contains("https://example.test/source"));
}

#[test]
fn system_part_body_accepts_named_interface_and_individual_members() {
    let input = "package P {\npart def Apollo11MissionSystem :> SystemOfSystems {\nindividual part launchVehicle : 'SA-506' :> constituentSystems;\npart spacecraft : ApolloSpacecraft :> constituentSystems {\nindividual part csm : 'CSM-107' :>> commandServiceModule;\nindividual part lm : 'LM-5' :>> lunarModule;\n}\npart spaceSuits[2] : ExtravehicularMobilityUnit :> constituentSystems;\ninterface lvToPayload : LVPayloadInterface connect launchVehicle.instrumentUnit.payloadInterfacePort to spacecraft.spacecraftLMAdapter.launchVehicleInterfacePort;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let system = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &system.body else {
        panic!("expected part body");
    };
    let spacecraft = match &elements[1].value {
        PartDefBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected part usage"),
    };
    let PartUsageBody::Brace {
        elements: spacecraft_elements,
    } = &spacecraft.body
    else {
        panic!("expected nested part body");
    };
    let csm = match &spacecraft_elements[0].value {
        PartUsageBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected individual part usage"),
    };
    assert_eq!(csm.name, "csm");
    assert!(csm.is_individual);
    assert_eq!(csm.type_name, "CSM-107");
    assert_eq!(csm.redefines.as_deref(), Some("commandServiceModule"));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::InterfaceUsage(_))));
}

#[test]
fn part_defs_accept_multiple_specialization_targets() {
    let input = "package P {\npart def ApolloSpacecraft :> System, Spacecraft {\npart commandServiceModule;\n}\npart def ExtravehicularMobilityUnit :> System, EVASystem;\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    assert!(matches!(elements[0].value, PackageBodyElement::PartDef(_)));
    assert!(matches!(elements[1].value, PackageBodyElement::PartDef(_)));
}

#[test]
fn part_redefinition_value_parses_parenthesized_tuple_of_engines() {
    let input = "package P {\npart def SII :> RocketStage {\npart engine1;\npart engine2;\npart engine3;\npart engine4;\npart engine5;\npart :>> engines[5] = (engine1, engine2, engine3, engine4, engine5);\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let sii = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &sii.body else {
        panic!("expected part body");
    };
    let engines = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::PartUsage(p) if p.value.redefines.as_deref() == Some("engines") => {
                Some(&**p)
            }
            _ => None,
        })
        .expect("engines part usage");
    assert_eq!(engines.value.multiplicity.as_deref(), Some("[5]"));
    let value = engines
        .value
        .value
        .as_ref()
        .expect("tuple value should parse");
    let Expression::Tuple(items) = &value.value else {
        panic!("expected Expression::Tuple, got {:?}", value.value);
    };
    assert_eq!(items.len(), 5);
    let expected = ["engine1", "engine2", "engine3", "engine4", "engine5"];
    for (i, name) in expected.iter().enumerate() {
        assert!(
            matches!(&items[i].value, Expression::FeatureRef(s) if s == name),
            "element {i}: expected FeatureRef({name:?}), got {:?}",
            items[i].value
        );
    }
}

#[test]
fn part_def_attribute_redefinition_usage_keeps_redefines_and_value() {
    let input = "package P {\npart def RocketStage {\nattribute propellantMass :> ISQ::mass;\nattribute dryMass :> ISQ::mass;\n}\npart def S_IC :> RocketStage {\nattribute :>> propellantMass = 2077000 [kg];\nattribute :>> dryMass = 137000 [kg];\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let sic = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("S_IC") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("expected S_IC part def");
    let PartDefBody::Brace { elements } = &sic.body else {
        panic!("expected part body");
    };

    let attrs: Vec<_> = elements
        .iter()
        .filter_map(|e| match &e.value {
            PartDefBodyElement::AttributeUsage(attr) => Some(&attr.value),
            _ => None,
        })
        .collect();
    assert_eq!(
        attrs.len(),
        2,
        "expected both attribute redefinitions as usages"
    );
    assert_eq!(attrs[0].name, "propellantMass");
    assert_eq!(attrs[0].redefines.as_deref(), Some("propellantMass"));
    assert!(
        attrs[0].value.is_some(),
        "propellantMass should keep assigned value"
    );
    assert_eq!(attrs[1].name, "dryMass");
    assert_eq!(attrs[1].redefines.as_deref(), Some("dryMass"));
    assert!(
        attrs[1].value.is_some(),
        "dryMass should keep assigned value"
    );
}
