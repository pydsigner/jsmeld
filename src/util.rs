use swc_ecma_ast::EsVersion;
use crate::errors::{JSMeldResult, JSMeldError};

pub fn parse_es_version(s: String) -> JSMeldResult<EsVersion> {
    match s.to_lowercase().as_str() {
        "es3" => Ok(EsVersion::Es3),
        "es5" => Ok(EsVersion::Es5),
        "es2015" | "es6" => Ok(EsVersion::Es2015),
        "es2016" => Ok(EsVersion::Es2016),
        "es2017" => Ok(EsVersion::Es2017),
        "es2018" => Ok(EsVersion::Es2018),
        "es2019" => Ok(EsVersion::Es2019),
        "es2020" => Ok(EsVersion::Es2020),
        "es2021" => Ok(EsVersion::Es2021),
        "es2022" => Ok(EsVersion::Es2022),
        "es2023" => Ok(EsVersion::Es2023),
        "es2024" => Ok(EsVersion::Es2024),
        "esnext" => Ok(EsVersion::EsNext),
        _ => Err(JSMeldError::ConfigError(format!("Unknown ES version: {s}"))),
    }
}
