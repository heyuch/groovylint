
            synchronized(data) {
                while(dataFile.hasRecords()) {
                    doStuff()
                    while(!data.isReady()) {
                        data.wait()
                    }
                    processData()
                }
            }
        