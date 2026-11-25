
            class MyClass {
                def myMethod() {
                    if (count > maxCount) {                     // 1
                        myList.each { element->                 // 2
                            sleep(1000L)
                            listeners.each {
                                notify(it)                      // 3
                                if (logging) {                  // 4
                                    if (loggingFilename) {      // 5
                                        element.values.each {   // 6
                                            println "value=$it"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            