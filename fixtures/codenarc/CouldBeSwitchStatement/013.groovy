
            void doStuff() {
                if (platform == 'win') {
                    println 'do some Windows stuff'
                } else {
                    println 'do some Mac or Linux stuff'
                }
    
                // This code in between should prevent a violation        
                println 'significant chunk of code common to all platforms'
            
                if (platform == 'linux') {
                    println 'do some Linux stuff in the subdir'
                } else {
                    println 'do some Mac or Windows stuff in the subdir'
                }
                
                if (platform == 'mac') {
                    println 'do some Mac stuff'
                } else {
                    println 'other ...'
                }
            }
        