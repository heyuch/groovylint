
            void doStuff() {
                if (platform == 'win') {
                    println 'do some Windows stuff'
                } else {
                    println 'do some Mac or Linux stuff'
                }
            
                dir('subdir') {
                    if (platform == 'linux') {
                        println 'do some Linux stuff in the subdir'
                    } else {
                        println 'do some Mac or Windows stuff in the subdir'
                    }
                }
                
                if (platform == 'mac') {
                    println 'do some Mac-specific postprocessing'
                }
            }
        