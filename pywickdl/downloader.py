import pywickdl
import asyncio

def make_future(loop):
    fut = loop.create_future()
    def cb(res):
        if isinstance(res, Exception):
            fut.set_exception(res)
        else:
            fut.set_result(res)
    return (fut, cb)
        

class WickDownloader:
    def __init__(self):
        self.loop = asyncio.get_running_loop()
        self.runtime = pywickdl.Runtime()
        self.service = None

    def __start_service(self):
        (fut, cb) = make_future(self.loop)
        self.runtime.create_service(self.loop, cb)
        return fut

    async def start_service(self):
        service = await self.__start_service()
        self.service = service

    def get_paks(self):
        return self.service.get_paks()

    def download_pak(self, pakname, target):
        (fut, cb) = make_future(self.loop)
        self.service.download_pak(self.runtime, pakname, target, self.loop, cb)
        return fut

    def get_pak(self, pakname):
        (fut, cb) = make_future(self.loop)
        self.service.get_pak(self.runtime, pakname, self.loop, cb)
        return fut

    def decrypt_pak(self, pak, key):
        (fut, cb) = make_future(self.loop)
        pak.decrypt(self.runtime, self.service, key, self.loop, cb)
        return fut

    def get_file_data(self, pak, file):
        (fut, cb) = make_future(self.loop)
        pak.get_file_data(self.runtime, file, self.loop, cb)
        return fut