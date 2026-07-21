"""V201-V250 真生产 batch tests (主 23:23)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
import importlib

MODULES = [
    "v201_python_asyncio", "v202_python_concurrent", "v203_python_multiprocessing",
    "v204_python_threading", "v205_python_asyncio_queues", "v206_python_threading_lock",
    "v207_python_signal", "v208_python_subprocess", "v209_python_socket",
    "v210_python_ssl", "v211_python_hashlib", "v212_python_secrets",
    "v213_python_hmac", "v214_python_zlib", "v215_python_gzip",
    "v216_python_lzma", "v217_python_bz2", "v218_python_lz4",
    "v219_python_zstd", "v220_python_zipfile", "v221_python_tarfile",
    "v222_python_csv", "v223_python_json", "v224_python_pickle",
    "v225_python_msgpack", "v226_python_protobuf", "v227_python_avro",
    "v228_python_parquet", "v229_python_redis", "v230_python_memcached",
    "v231_python_kafka", "v232_python_grpc", "v233_python_rest",
    "v234_python_graphql", "v235_python_websocket", "v236_python_prometheus",
    "v237_python_opentelemetry", "v238_python_pydantic", "v239_python_dataclasses",
    "v240_python_asyncio_gather", "v241_python_subprocess_popen", "v242_python_asyncio_subprocess",
    "v243_python_signal_handlers", "v244_python_thread_pool", "v245_python_process_pool",
    "v246_python_asyncio_queue", "v247_python_asyncio_event", "v248_python_asyncio_lock",
    "v249_python_asyncio_semaphore", "v250_python_asyncio_master",
]


@pytest.mark.parametrize("mod_name", MODULES)
def test_module_init(mod_name):
    mod = importlib.import_module(f"apeireth.{mod_name}")
    # 找 class
    class_name = "V" + mod_name.split("_")[0][1:] + "Module"
    cls = getattr(mod, class_name, None)
    assert cls is not None
    obj = cls()
    assert obj is not None