"""The provider list in every README must match the runtime catalog."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
READMES = ("README.md", "README.vi.md", "README.ko.md", "README.zh.md")
LIST_SEPARATORS = r"[,、]"


def catalog_names() -> list[str]:
    source = (REPO / "src/model/catalog.rs").read_text(encoding="utf-8")
    return re.findall(r'(?m)^\s*name:\s*"([^"]+)"', source)


def readme_names(text: str) -> list[str]:
    """Provider ids on the "Providers" bullet, up to the link to the catalog."""
    line = next((row for row in text.splitlines() if row.startswith("- **Provider")), "")
    listed = line.split("**", 2)[-1].split("[`src/model/catalog.rs`]")[0]
    # Drop the "(cloud)" / "(local)" notes and the sentence that follows the lists.
    listed = re.split(r"\.|。", re.sub(r"[(（][^)）]*[)）]", ",", listed))[0]
    names = []
    for part in re.split(LIST_SEPARATORS, listed):
        ids = re.findall(r"[a-z0-9]+", part)
        if ids:
            # A joining word such as "and" or "và" can precede the id.
            names.append(ids[-1])
    return names


class ReadmeProviderTests(unittest.TestCase):
    def test_catalog_is_readable(self) -> None:
        self.assertGreater(len(catalog_names()), 0)

    def test_every_readme_lists_exactly_the_catalog(self) -> None:
        expected = sorted(catalog_names())
        for name in READMES:
            with self.subTest(readme=name):
                text = (REPO / name).read_text(encoding="utf-8")
                self.assertEqual(sorted(readme_names(text)), expected)

    def test_missing_bullet_yields_no_names(self) -> None:
        self.assertEqual(readme_names(""), [])

    def test_extra_provider_is_reported(self) -> None:
        line = "- **Providers:** anthropic, ghost (cloud), and ollama (local). Done."
        self.assertEqual(readme_names(line), ["anthropic", "ghost", "ollama"])


if __name__ == "__main__":
    unittest.main()
