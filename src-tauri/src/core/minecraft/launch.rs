use super::rules::TargetOs;

/// Separador de classpath da JVM: `;` no Windows, `:` no resto.
pub fn classpath_separator(os: TargetOs) -> char {
    match os {
        TargetOs::Windows => ';',
        TargetOs::Linux | TargetOs::MacOs => ':',
    }
}

pub fn build_classpath(jar_paths: &[String], os: TargetOs) -> String {
    jar_paths.join(&classpath_separator(os).to_string())
}

/// Argv completo pra invocar a JVM (sem contar o executável java em
/// si, que é o `Command::new`): flags da JVM, depois a main class,
/// depois os argumentos de jogo — nessa ordem a JVM exige.
pub fn build_launch_argv(jvm_args: &[String], main_class: &str, game_args: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(jvm_args.len() + 1 + game_args.len());
    argv.extend(jvm_args.iter().cloned());
    argv.push(main_class.to_string());
    argv.extend(game_args.iter().cloned());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classpath_uses_semicolon_on_windows_and_colon_elsewhere() {
        let jars = vec!["a.jar".to_string(), "b.jar".to_string()];
        assert_eq!(build_classpath(&jars, TargetOs::Windows), "a.jar;b.jar");
        assert_eq!(build_classpath(&jars, TargetOs::Linux), "a.jar:b.jar");
        assert_eq!(build_classpath(&jars, TargetOs::MacOs), "a.jar:b.jar");
    }

    #[test]
    fn launch_argv_orders_jvm_args_then_main_class_then_game_args() {
        let jvm = vec!["-Xmx4G".to_string()];
        let game = vec!["--username".to_string(), "BDarkBr".to_string()];
        let argv = build_launch_argv(&jvm, "net.minecraft.client.main.Main", &game);
        assert_eq!(
            argv,
            vec!["-Xmx4G", "net.minecraft.client.main.Main", "--username", "BDarkBr"]
        );
    }
}
