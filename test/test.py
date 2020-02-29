import pywickdl
import asyncio

async def main():
    downloader = pywickdl.WickDownloader()
    await downloader.start_service()
    pakname = downloader.get_paks()[0]
    encpak = await downloader.get_pak(pakname)
    pak = await downloader.decrypt_pak(encpak, "pak key")
    firstname = pak.get_file_names()[0]
    print(firstname)


asyncio.run(main())