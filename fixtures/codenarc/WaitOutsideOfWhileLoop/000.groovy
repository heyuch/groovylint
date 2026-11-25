
            synchronized(data) {
                while (!data.isReady()) {
                    data.wait()
                }
                data.calculateStatistics()
            }
        