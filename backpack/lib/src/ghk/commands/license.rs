use crate::cli::LicenseKind;
use crate::ghk::{gh, git, util};
use anyhow::{Result, bail};
use dialoguer::Select;
use std::fs;
use chrono::Datelike;

pub fn run(kind: Option<LicenseKind>) -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    if std::path::Path::new("LICENSE").exists() {
        util::warn("LICENSE file already exists");
        return Ok(());
    }

    let license = match kind {
        Some(ref k) => k,
        None => {
            let options = ["MIT", "Apache 2.0", "GPL 3.0", "Unlicense"];
            let idx = Select::new()
                .with_prompt("Choose license")
                .items(&options)
                .default(0)
                .interact()?;
            &match idx {
                0 => LicenseKind::Mit,
                1 => LicenseKind::Apache,
                2 => LicenseKind::Gpl,
                _ => LicenseKind::Unlicense,
            }
        }
    };

    let year = chrono::Local::now().year();
    let author = gh::whoami().unwrap_or_else(|_| "Your Name".to_string());

    let content = match license {
        LicenseKind::Mit => format!(
            "MIT License\n\nCopyright (c) {} {}\n\n\
            Permission is hereby granted, free of charge, to any person obtaining a copy\n\
            of this software and associated documentation files (the \"Software\"), to deal\n\
            in the Software without restriction, including without limitation the rights\n\
            to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
            copies of the Software, and to permit persons to whom the Software is\n\
            furnished to do so, subject to the following conditions:\n\n\
            The above copyright notice and this permission notice shall be included in all\n\
            copies or substantial portions of the Software.\n\n\
            THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n\
            IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n\
            FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n\
            AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n\
            LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n\
            OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n\
            SOFTWARE.\n", year, author
        ),
        LicenseKind::Apache => format!(
            "Copyright {} {}\n\n\
            Licensed under the Apache License, Version 2.0 (the \"License\");\n\
            you may not use this file except in compliance with the License.\n\
            You may obtain a copy of the License at\n\n\
                http://www.apache.org/licenses/LICENSE-2.0\n\n\
            Unless required by applicable law or agreed to in writing, software\n\
            distributed under the License is distributed on an \"AS IS\" BASIS,\n\
            WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.\n\
            See the License for the specific language governing permissions and\n\
            limitations under the License.\n", year, author
        ),
        LicenseKind::Gpl => format!(
            "Copyright (C) {} {}\n\n\
            This program is free software: you can redistribute it and/or modify\n\
            it under the terms of the GNU General Public License as published by\n\
            the Free Software Foundation, either version 3 of the License, or\n\
            (at your option) any later version.\n\n\
            This program is distributed in the hope that it will be useful,\n\
            but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
            MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the\n\
            GNU General Public License for more details.\n\n\
            You should have received a copy of the GNU General Public License\n\
            along with this program. If not, see <https://www.gnu.org/licenses/>.\n", year, author
        ),
        LicenseKind::Unlicense => String::from(
            "This is free and unencumbered software released into the public domain.\n\n\
            Anyone is free to copy, modify, publish, use, compile, sell, or\n\
            distribute this software, either in source code form or as a compiled\n\
            binary, for any purpose, commercial or non-commercial, and by any means.\n\n\
            In jurisdictions that recognize copyright laws, the author or authors\n\
            of this software dedicate any and all copyright interest in the\n\
            software to the public domain.\n\n\
            THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND.\n"
        ),
    };

    fs::write("LICENSE", content)?;
    util::ok("Created LICENSE file");

    Ok(())
}
