use common_macros::hash_map;

use crate::Expression;
use os_info::Type;
use smallstr::SmallString;

fn get_os_family(t: &Type) -> String {
    match t {
        Type::Amazon | Type::Android => "android",
        Type::Alpaquita
        | Type::Alpine
        | Type::Arch
        | Type::Artix
        | Type::Bluefin
        | Type::CachyOS
        | Type::CentOS
        | Type::Debian
        | Type::EndeavourOS
        | Type::Fedora
        | Type::Gentoo
        | Type::Linux
        | Type::Manjaro
        | Type::Mariner
        | Type::NixOS
        | Type::Nobara
        | Type::Uos
        | Type::OpenCloudOS
        | Type::openEuler
        | Type::openSUSE
        | Type::OracleLinux
        | Type::Pop
        | Type::Redhat
        | Type::RedHatEnterprise
        | Type::SUSE
        | Type::Ubuntu
        | Type::Ultramarine
        | Type::Void
        | Type::Mint => "linux",

        Type::AIX | Type::Macos | Type::Solus | Type::Redox => "unix",

        Type::Windows => "windows",
        Type::Emscripten => "WebAssembly",
        Type::Unknown | _ => "unknown",
    }
    .to_string()
}

pub fn get() -> Expression {
    let os = os_info::get();
    let os_type = os.os_type();

    (hash_map! {
       SmallString::from("name") => Expression::String(os_type.to_string()),
       SmallString::from("family") => get_os_family(&os_type).into(),
       SmallString::from("version") => os.version().to_string().into(),

    })
    .into()
}
