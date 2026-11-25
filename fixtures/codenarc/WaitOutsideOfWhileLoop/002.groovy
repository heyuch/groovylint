
            synchronized(data) {
                data.wait()
                data.calculateStatistics()
            }
        