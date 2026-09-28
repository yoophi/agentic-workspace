import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-process-spawn-inventory.py"
SPEC = importlib.util.spec_from_file_location("process_inventory", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class InventoryNegativeFixtures(unittest.TestCase):
    def test_aliased_command_constructor_is_counted(self):
        source = """
use std::process::Command as HiddenCommand;
fn launch() { let _ = HiddenCommand::new("git"); }
"""
        self.assertEqual(MODULE.command_creation_count(source), 1)

    def test_an_extra_constructor_in_an_existing_file_changes_the_baseline(self):
        source = """
use std::process::Command;
fn first() { let _ = Command::new("git"); }
fn added_later() { let _ = Command::new("curl"); }
"""
        actual = MODULE.command_creation_count(source)
        self.assertEqual(actual, 2)
        self.assertNotEqual(actual, 1, "a same-file added spawn must drift")

    def test_cfg_test_module_is_removed_but_following_production_code_remains(self):
        source = """
#[cfg(test)]
mod tests { fn fixture() { let _ = std::process::Command::new("cat"); } }
fn production() { let _ = std::process::Command::new("git"); }
"""
        production = MODULE.without_cfg_test_items(source)
        self.assertEqual(MODULE.command_creation_count(production), 1)

    def test_cfg_test_import_does_not_remove_following_production_function(self):
        source = """
#[cfg(test)]
use crate::test_helpers;
fn production() { let _ = std::process::Command::new("git"); }
"""
        production = MODULE.without_cfg_test_items(source)
        self.assertNotIn("test_helpers", production)
        self.assertEqual(MODULE.command_creation_count(production), 1)


if __name__ == "__main__":
    unittest.main()
