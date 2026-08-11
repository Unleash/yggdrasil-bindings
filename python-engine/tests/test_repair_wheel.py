import subprocess
import sys
import zipfile
from pathlib import Path

REPAIR_WHEEL = Path(__file__).parent.parent / "scripts" / "repair_wheel.py"


def test_wheel_metadata_tag_matches_the_filename_tag(tmp_path: Path) -> None:
    (tmp_path / "dist").mkdir()
    (tmp_path / "staging").mkdir()
    source = tmp_path / "dist" / "yggdrasil_engine-1.3.2-py3-none-any.whl"
    with zipfile.ZipFile(source, "w") as wheel:
        wheel.writestr("yggdrasil_engine-1.3.2.dist-info/WHEEL", "Tag: py3-none-any\n")

    _ = subprocess.run(
        [sys.executable, str(REPAIR_WHEEL)], cwd=str(tmp_path), check=True
    )

    repaired = next((tmp_path / "staging").iterdir())
    with zipfile.ZipFile(repaired) as wheel:
        metadata = wheel.read("yggdrasil_engine-1.3.2.dist-info/WHEEL").decode()
    filename_tag = "-".join(repaired.stem.split("-")[-3:])
    assert [line for line in metadata.splitlines() if line.startswith("Tag:")] == [
        f"Tag: {filename_tag}"
    ]
