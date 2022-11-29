use std::{collections::HashMap, path::PathBuf, sync::Arc};

use farmfe_core::{
  config::Config,
  context::CompilationContext,
  module::{Module, ModuleType},
  plugin::{
    Plugin, PluginAnalyzeDepsHookParam, PluginAnalyzeDepsHookResultEntry, PluginHookContext,
    PluginLoadHookParam, PluginParseHookParam, ResolveKind,
  },
  resource::resource_pot::{
    JsResourcePotMetaData, ResourcePot, ResourcePotId, ResourcePotMetaData, ResourcePotType,
  },
  swc_common::DUMMY_SP,
  swc_ecma_ast::Module as SwcModule,
};
use farmfe_toolkit::testing_helpers::fixture;

#[test]
fn load_parse_and_analyze_deps() {
  fixture(
    "tests/fixtures/load_parse_analyze/**/index.*",
    |file: PathBuf, _| {
      let config = Config::default();
      let plugin_script = farmfe_plugin_script::FarmPluginScript::new(&config);
      let context = Arc::new(CompilationContext::new(config, vec![]).unwrap());
      let id = file.to_string_lossy().to_string();
      let hook_context = PluginHookContext {
        caller: None,
        meta: HashMap::new(),
      };
      let loaded = plugin_script.load(
        &PluginLoadHookParam {
          resolved_path: &id,
          query: HashMap::new(),
        },
        &context,
        &hook_context,
      );

      assert!(loaded.is_ok());
      let loaded = loaded.unwrap();
      assert!(loaded.is_some());
      let loaded = loaded.unwrap();

      let lines: Vec<&str> = loaded.content.lines().collect();
      assert_eq!(
        lines,
        vec![
          "import a from './a';",
          "import b from './b';",
          "",
          "console.log(a, b);"
        ]
      );
      assert_eq!(
        loaded.module_type,
        match file.extension().unwrap().to_str().unwrap() {
          "js" => ModuleType::Js,
          "jsx" => ModuleType::Jsx,
          "ts" => ModuleType::Ts,
          "tsx" => ModuleType::Tsx,
          _ => unreachable!("never be here"),
        }
      );

      let module_meta = plugin_script
        .parse(
          &PluginParseHookParam {
            module_id: "any".into(),
            resolved_path: id,
            query: HashMap::new(),
            module_type: loaded.module_type.clone(),
            content: loaded.content,
          },
          &context,
          &hook_context,
        )
        .unwrap()
        .unwrap();

      let mut module = Module::new("any".into());
      module.meta = module_meta;
      module.module_type = loaded.module_type;

      assert_eq!(module.meta.as_script().ast.body.len(), 3);

      let mut deps = PluginAnalyzeDepsHookParam {
        module: &module,
        deps: vec![],
      };
      plugin_script
        .analyze_deps(&mut deps, &context)
        .unwrap()
        .unwrap();
      assert_eq!(
        deps.deps,
        vec![
          PluginAnalyzeDepsHookResultEntry {
            source: String::from("./a"),
            kind: ResolveKind::Import
          },
          PluginAnalyzeDepsHookResultEntry {
            source: String::from("./b"),
            kind: ResolveKind::Import
          }
        ]
      );

      let mut resource_pot = ResourcePot::new(
        ResourcePotId::new("index".to_string()),
        ResourcePotType::Js,
        "any".into(),
      );

      resource_pot.resource_pot_type = ResourcePotType::Js;
      resource_pot.meta = ResourcePotMetaData::Js(JsResourcePotMetaData {
        ast: SwcModule {
          body: module.meta.as_script().ast.body.to_vec(),
          shebang: None,
          span: DUMMY_SP,
        },
      });

      let resources = plugin_script
        .generate_resources(&mut resource_pot, &context, &hook_context)
        .unwrap()
        .unwrap();
      assert_eq!(resources.len(), 1);

      let code = String::from_utf8(resources[0].bytes.clone()).unwrap();
      let lines: Vec<&str> = code.lines().collect();
      assert_eq!(
        lines,
        vec![
          "import a from \"./a\";",
          "import b from \"./b\";",
          "console.log(a, b);"
        ]
      );
    },
  );
}