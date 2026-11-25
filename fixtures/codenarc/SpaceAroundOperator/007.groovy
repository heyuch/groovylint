
                def x = alert.isInternal()
                        ? planInfo.isMatchingPlan() ? LOGO_URL_1        // nested ternary
                        : LOGO_URL_2
                        : LOGO_URL_1
    
                def y = alert.isInternal()
                        ? LOGO_URL_1
                        : LOGO_URL_2
    
                def z = alert.isInternal() ?
                        LOGO_URL_1 :
                        LOGO_URL_2
            