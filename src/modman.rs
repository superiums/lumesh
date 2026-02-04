use crate::MAX_USEMODE_RECURSION;
use crate::SyntaxErrorKind;
use crate::utils::canon;
use crate::utils::expand_home;
use crate::{Environment, Expression, ModuleInfo, RuntimeError, SyntaxError, use_script};
use std::borrow::Cow;
// use std::collections::HashSet;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub fn use_module<'a>(
    alias: &'a Option<String>,
    module_path: &'a str,
    env: &mut Environment,
) -> Result<(Cow<'a, str>, Expression), RuntimeError> {
    // 获取基础路径
    let base = match env.get("SCRIPT") {
        Some(Expression::String(s)) => s,
        _ => String::from("."),
    };
    let cwd = Path::new(&base);
    use_module_wrap(alias, module_path, cwd, env, 0)
}
pub fn use_module_wrap<'a>(
    alias: &'a Option<String>,
    module_path: &'a str,
    base: &Path,
    env: &mut Environment,
    use_depth: usize,
) -> Result<(Cow<'a, str>, Expression), RuntimeError> {
    let (module_info, parent_file) = load_module(module_path, base, env)?;
    let mut map = module_info.functions;

    if !module_info.use_statements.is_empty() {
        let cwd = parent_file.parent().unwrap_or(base);
        // 递归use语句
        // let mut already = HashSet::new();
        for (ua, up) in module_info.use_statements.iter() {
            // 避免循环调用
            if MAX_USEMODE_RECURSION.with_borrow(|v| &use_depth < v) {
                // 允许重复调用,但给出提示
                // if !already.insert(ua){
                //     eprintln!()
                // }
                let (na, np) = use_module_wrap(ua, up, cwd, env, use_depth + 1)?;
                map.insert(na.into(), np);
            }
        }
    }

    // 使用别名或模块名作为键，存储为Map
    let module_name = get_module_name_from_path(&alias, module_path)?;
    Ok((module_name, Expression::from(map)))
}
fn load_module(
    file_path: &str,
    cwd: &Path,
    env: &mut Environment,
) -> Result<(ModuleInfo, PathBuf), RuntimeError> {
    let mod_file = find_module_file(file_path, cwd, env)?;
    Ok((read_module_file(&mod_file, env)?, mod_file))
}
fn find_module_file(
    file_path: &str,
    cwd: &Path,
    env: &mut Environment,
) -> Result<PathBuf, RuntimeError> {
    // 构建文件名（统一处理扩展名和路径）
    let file = Path::new(expand_home(file_path).as_ref()).with_extension("lm");
    let modname = file
        .file_prefix()
        .and_then(|x| x.to_str())
        .unwrap_or("uknown");
    // 预构建所有候选路径
    let lib = match env.get("LUME_MODULES_PATH") {
        Some(Expression::String(mo)) => Path::new(&mo).to_path_buf(),
        _ => dirs::data_local_dir()
            .unwrap_or(PathBuf::from("~/.local/share"))
            .join("lumesh/mods"),
    };

    let candidate_paths = vec![
        cwd.join("mods").join(&file),
        cwd.join("mods").join(&modname).join("main.lm"),
        cwd.join(&file),
        cwd.join(&modname).join("main.lm"),
        lib.join(&file),
        lib.join(&modname).join("main.lm"),
    ];

    // 使用 iter() 和 find_map() 查找第一个有效路径
    let mod_file = candidate_paths
        .iter()
        .find_map(|path| {
            let path_str = path.to_str().unwrap_or_default();
            canon(path_str, env).ok()
        })
        .ok_or_else(|| {
            RuntimeError::common(
                format!("module `{file_path}` not found in:\n{:?}", candidate_paths).into(),
                Expression::String(file.to_string_lossy().into()),
                0,
            )
        })?;
    Ok(mod_file)
}
fn read_module_file(
    mod_file: &PathBuf,
    _env: &mut Environment,
) -> Result<ModuleInfo, RuntimeError> {
    // 读取并解析模块文件
    let module_content = match read_to_string(mod_file) {
        Ok(content) => content,
        Err(e) => {
            return Err(RuntimeError::from_io_error(
                e,
                "loading module".into(),
                Expression::None,
                0,
            ));
        }
    };

    // 解析模块内容
    match use_script(&module_content) {
        Ok(result) => Ok(result),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            let err = SyntaxError {
                source: format!("{module_content}   ").into(),
                kind: e,
            };
            Err(RuntimeError::common(
                err.to_string().into(),
                Expression::None,
                0,
            ))
        }
        Err(nom::Err::Incomplete(_)) => {
            let err = SyntaxError {
                source: module_content.into(),
                kind: SyntaxErrorKind::InternalError("incompleted".to_string()),
            };
            Err(RuntimeError::common(
                err.to_string().into(),
                Expression::None,
                0,
            ))
        }
    }
}

fn get_module_name_from_path<'a>(
    alias: &'a Option<String>,
    module_path: &'a str,
) -> Result<Cow<'a, str>, RuntimeError> {
    match alias {
        Some(n) => Ok(n.into()),
        _ => {
            let path = Path::new(module_path);

            // 获取文件名
            match path.file_name() {
                Some(name) => {
                    let fname = name.to_string_lossy();
                    Ok(match fname.split_once('.') {
                        Some((n, _)) => n.to_string().into(),
                        _ => fname.to_string().into(),
                    })
                }
                None => Err(RuntimeError::common(
                    "get filename failed".into(),
                    Expression::Use(alias.clone(), module_path.to_string()),
                    0,
                )),
            }
        }
    }
}
