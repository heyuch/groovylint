
    @Test
            void testProcess() {
                shouldFail { process('xxx://wrsnetdev.usa.wachovia.net') }
                shouldFail(Exception) { process('http://')
                }
                shouldFail {
                    process('http://wrsnetdev.usa.wachovia.net:xxx') }
            }
        