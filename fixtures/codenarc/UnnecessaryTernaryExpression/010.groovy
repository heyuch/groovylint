
            transactionTypes.equals('ALL') ?
                myService.findAllWidgetsByPlan{ plan -> valueOne } :
                myService.findAllWidgetsByPlan{ plan -> valueTwo }
        