
            class InvalidClass {
                def methodWithClosures() {
                    def list = []
                    list.each { item ->

                        println
                    }

                    list.each {

                        1
                    }
                }
            }
        