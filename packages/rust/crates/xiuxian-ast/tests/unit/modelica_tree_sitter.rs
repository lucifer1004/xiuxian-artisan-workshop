use super::TreeSitterModelicaParser;

#[test]
fn parse_modelica_file_summary() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(mut parser) = TreeSitterModelicaParser::new() else {
        return Ok(());
    };

    let code = r"
package MyPackage
  model MyModel
    Real y;
  equation
    y = 10 * 2;
  end MyModel;
end MyPackage;
";

    let summary = parser.parse_file_summary(code)?;
    assert_eq!(summary.symbols[0].equations.len(), 1);
    assert!(summary.symbols[0].equations[0].contains("y = 10 * 2"));
    Ok(())
}
