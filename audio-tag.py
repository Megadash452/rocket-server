#!/usr/bin/python3
import mutagen # pip install mutagen; https://github.com/quodlibet/mutagen
from mutagen import MutagenError
from mutagen.flac import Picture
from mutagen.mp3 import EasyMP3
from mutagen.id3 import PictureType, ID3, APIC
from mutagen._vorbis import VComment as VorbisComment
import magic # pip install python-magic; https://github.com/ahupp/python-magic
from pathlib import Path
import json as JSON
import base64
import datetime
import sys
import os


help = """Command-Line tool for Audio Tags
Usage:
    ./audio-tag.py [FLAGS] [OPTIONS] <FILE_PATHS>
    ./audio-tag.py SUBCOMMAND OPTIONS <FILE_PATHS>
    if a FILE_PATH is a directory, uses all the files in that directory (not recursively).
Options:
  --export-covers-dir <PATH>: Export the Album/Cover Art of each file (if it has one)
                              to the specified directory.
Flags:
  -j, --json: Print info as JSON for parsing.
  -u, --update-covers: Only write to the Album/cover Art file if the audio file changed
                       since last export (saves time and power).
  -h, --help: Print this message.
Subcommands:
  set: Instead of outputting tag data, write data to the provided audio files.
    Options:
      --title, --artist, --album, --album-artist, --release-year, --track-number.
      --cover: Import an image file as the Album/Cover Art.
      Options take 1 value
    Flags:
      -r, --remove-cover: Remove the stored Album/cover Art(s).
                          Mutually exclusive with --cover-path.
"""

IMAGE_EXTS = {
    "image/jpeg": "jpg",
    "image/png": "png",
}
IMAGE_MIMES = {v: k for k, v in IMAGE_EXTS.items()}

cover_update_write = False
paths: list[Path] = []

def iter_args(args: list[str], flag_handler = None, option_handler = None):
    i = 0
    while i < len(args):
        arg = args[i]
        if arg[0:2] == "--":
            arg = arg[2:]
            try:
                if flag_handler is not None:
                    flag_handler(arg)
                else:
                    raise KeyError
            except KeyError:
                # Is not Flag, is Option
                try:
                    if i + 1 < len(args):
                        if option_handler is not None:
                            option_handler(arg, args[i + 1])
                        i += 1
                    else:
                        print(f"Provide a value for Option \"{arg}\"", file=sys.stderr)
                        exit(1)
                except KeyError:
                    print(f"Unknown Option or Flag: \"{arg}\"", file=sys.stderr)
                    exit(1)
        elif arg[0] == '-':
            for flag in arg[1:]:
                try:
                    if flag_handler is not None:
                        flag_handler(flag)
                except KeyError:
                    print(f"Unknown Flag: \"{flag}\"", file=sys.stderr)
                    exit(1)
        else:
            paths.append(Path(arg))
            
        i += 1

def do_per_file(paths: list[Path], handler, *args):
    if len(paths) == 1:
        # Only set exit code Success/Error when dealing with only 1 file
        file_path = paths[0]
        if file_path.exists() and file_path.is_file():
            exit(handler(paths[0], *args))
        elif not file_path.exists():
            # Trying to open a non-existing file will yield OsError
            try:
                open(file_path)
            except OSError as err:
                print(err, file=sys.stderr)
                exit(err.errno)
            

    for i, path in enumerate(paths):
        if path.is_dir():
            # Don't export album covers (even if flag is set) when is dir
            old_cover_export_dir = cover_export_dir
            cover_export_dir = None
            for file_path in file_path.iterdir():
                if file_path.is_file():
                    handler(file_path, *args)
                    print("")
            cover_export_dir = old_cover_export_dir
        else:
            handler(path, *args)
        if i < len(paths) - 1:
            print("")


def write_export(export_path: Path | str, audio_path: Path, data: bytes):
    try:
        # Don't write the export unless audio was modified since last export
        if os.path.getmtime(audio_path) < os.path.getmtime(export_path) and cover_update_write:
            return
        export = open(export_path, "wb")
    except FileNotFoundError:
        Path(export_path).parents[0].mkdir(parents=True, exist_ok=True)
        export = open(export_path, "wb")

    export.write(data)

### Opens a file, ensuring that the file and tag are valid (i.e. not None).
def open_file(file_path: Path) -> mutagen.File:
    try:
        file = mutagen.File(file_path)
    except (FileNotFoundError, MutagenError) as e:
        raise type(e)(f"{type(e)}:", e)
    if file is None:
        raise ValueError("Can't determine file type")
    tag = file.tags
    if tag is None:
        raise ValueError("Could not load tag")
    return file


def tag_info(file_path: Path, cover_export_dir: Path = None, as_json = False):
    try:
        file = open_file(file_path)
    except Exception as e:
        print(f"Error with \"{file_path}\":", e, file=sys.stderr)
        return 1
    tag = file.tags

    pictures = []
    if isinstance(tag, ID3):
        if cover_export_dir is not None:
            pictures = tag.getall("APIC")
        file = EasyMP3(file_path)
        tag = file.tags
    elif isinstance(tag, VorbisComment):
        if cover_export_dir is not None:
            for i, picture_data in enumerate(tag["METADATA_BLOCK_PICTURE"]):
                try:
                    data = base64.b64decode(picture_data)
                except (TypeError, ValueError):
                    continue
                try:
                    pictures.append(Picture(data))
                except mutagen.flac.error:
                    continue
    else:
        print(f"Unknown File Type for \"{file_path}\"", file=sys.stderr)
        return 1
    
    if cover_export_dir is not None:
        cover_export_path = cover_export_dir.joinpath(file_path.name)

        for i, picture in enumerate(pictures):
            ext = IMAGE_EXTS.get(picture.mime, "jpg")
            if i == 0:
                export_path = f"{cover_export_path}.{ext}"
            else:
                export_path = f"{cover_export_path}-{i}.{ext}"
            write_export(export_path, file_path, picture.data)

    output = {}
    output["file"] = str(file_path)
    try:
        output["title"] = tag["title"][0]
    except KeyError:
        pass
    try:
        output["album"] = tag["album"][0]
    except KeyError:
        pass
    try:
        output["artist"] = tag["artist"][0]
    except KeyError:
        pass
    try:
        output["album-artist"] = tag["albumartist"][0]
    except KeyError:
        pass
    try:
        output["release-year"] = int(tag["date"][0])
    except ValueError:
        output["release-year"] = int(tag["date"][0].split('-')[0])
    except KeyError:
        pass
    try:
        output["track-number"] = tag["tracknumber"][0] # "2" or "2/4"
    except KeyError:
        pass
    output["length"] = round(file.info.length) # in seconds

    if as_json:
        print(JSON.dumps(output))
    else:
        for k in output:
            if k == "length":
                print("length:", datetime.timedelta(seconds=output["length"])) # in time duration format
                continue
            print(f"{k}:", output[k])

### cover: (img_data: bytes, mime: str) | True (remove cover) | False (do nothing)
def set_tag_info(file_path: Path, info: dict, cover: tuple[bytes, str] | bool = False):
    try:
        file = open_file(file_path)
    except Exception as e:
        print(f"Error with \"{file_path}\":", e, file=sys.stderr)
        return 1
    tag = file.tags
    
    if isinstance(tag, ID3):
        if cover == True:
            file.tags.delall("APIC")
            file.save()
        elif cover != False:
            tag.add(APIC(
                type = PictureType.COVER_FRONT,
                mime = cover[1],
                data = cover[0]
            ))
            file.save()
        file = EasyMP3(file_path)
    elif isinstance(tag, VorbisComment):
        if cover == True:
            tag["METADATA_BLOCK_PICTURE"] = []
        elif cover != False:
            picture = Picture()
            picture.type = PictureType.COVER_FRONT
            picture.mime = cover[1]
            picture.data = cover[0]
            tag["METADATA_BLOCK_PICTURE"] = [base64.b64encode(picture.write()).decode("ascii")]
    else:
        print(f"Unknown File Type for \"{file_path}\"", file=sys.stderr)
        return 1

    for k in info:
        if k == "release-year":
            tag_k = "date"
        else:
            tag_k = k.replace('-', "")
        file.tags[tag_k] = info[k]

    file.save()



try:
    subcommand = sys.argv[1]
except IndexError:
    # No arguments
    print(help)
    exit(0)

if subcommand == "help":
    print(help)
    exit(0)
elif subcommand == "set":
    tag_edit = {}
    cover_path = None
    remove_cover = False
    def option_handler(option: str, val: str):
        global tag_edit
        global cover_path
        match option:
            case "title" | "artist" | "album" | "album-artist" | "release-year" | "track-number":
                tag_edit[option] = val
            case "cover":
                cover_path = Path(val)
            case _:
                raise KeyError
    def flag_handler(flag: str):
        global remove_cover
        match flag:
            case 'r' | "remove-cover":
                remove_cover = True
            case _:
                raise KeyError
            
    iter_args(sys.argv[2:], flag_handler, option_handler)

    if len(tag_edit) == 0 and cover_path is None and not remove_cover:
        print("Provide some options to edit", file=sys.stderr)
        exit(1)

    if cover_path is not None:
        if remove_cover:
            print("--remove-cover (-r) and --cover are mutually exclusive", file=sys.stderr)
            exit(1)
        mime = magic.from_file(cover_path, mime=True)
        if IMAGE_EXTS.get(mime) is None:
            print("Cover is not a JPG or PNG", file=sys.stderr)
            exit(1)
        try:
            with open(cover_path, 'rb') as cover:
                do_per_file(paths, set_tag_info, tag_edit, (cover.read(), mime))
        except FileNotFoundError as e:
            print(e, file=sys.stderr)
            exit(1)
    else:
        do_per_file(paths, set_tag_info, tag_edit, remove_cover)
else:
    as_json = False
    cover_export_dir = None
    # argv[1] is not a subcommand
    def option_handler(option: str, val: str):
        global cover_export_dir
        match option:
            case "export-covers-dir":
                cover_export_dir = Path(val)
            case _:
                raise KeyError   
    def flag_handler(flag: str):
        global as_json
        global cover_update_write
        match flag:
            case 'j' | "json":
                as_json = True
            case 'u' | "update-covers":
                cover_update_write = True
            case 'h' | "help":
                print(help)
                exit(0)
            case _:
                raise KeyError
    
    iter_args(sys.argv[1:], flag_handler, option_handler)
    do_per_file(paths, tag_info, cover_export_dir, as_json)
