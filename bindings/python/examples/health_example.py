from orderflow import DataQualityFlags, Engine, EngineConfig, StreamKind, Symbol


def on_health(event: dict) -> None:
    print("health event:", event)


def main() -> None:
    cfg = EngineConfig(instance_id="py-health-example")
    with Engine(cfg) as engine:
        symbol = Symbol("CME", "ESM6", depth_levels=10)
        engine.subscribe(symbol, StreamKind.HEALTH, callback=on_health)
        engine.subscribe(symbol, StreamKind.ANALYTICS)

        engine.poll_once()
        engine.poll_once(DataQualityFlags.ADAPTER_DEGRADED)
        engine.poll_once()
        engine.unsubscribe(symbol)

        print("metrics:", engine.metrics())


if __name__ == "__main__":
    main()
