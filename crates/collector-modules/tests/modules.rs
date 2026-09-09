use collector_core::*;
use collector_modules::*;
struct Stamps;
impl CollectionModule for Stamps {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.stamps".into()),
            schema_version: 1,
            display_name: "Stamps".into(),
            capabilities: vec![],
        }
    }
    fn validate(&self, data: &ModuleData) -> Result<()> {
        if data.payload["denomination"].is_number() {
            Ok(())
        } else {
            Err(Error::Invalid("denomination required".into()))
        }
    }
}
#[test]
fn third_module_requires_no_core_variant() {
    let mut registry = Registry::builtins();
    registry.register(Box::new(Stamps)).unwrap();
    assert!(
        registry
            .validate(&ModuleData {
                module: ModuleId("org.example.stamps".into()),
                schema_version: 1,
                payload: serde_json::json!({"denomination":20})
            })
            .is_ok()
    );
    assert!(registry.register(Box::new(Stamps)).is_err());
}
