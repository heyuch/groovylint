
            synchronized(data) {
                for(;!data.isReady();) {
                    data.wait()
                }
                data.calculateStatistics()
            }
        