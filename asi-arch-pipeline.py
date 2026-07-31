import asyncio

from agents import set_default_openai_api, set_default_openai_client, set_tracing_disabled
from openai import AsyncAzureOpenAI

from analyse import analyse
from database import program_sample, update
from eval import evaluation
from evolve import evolve
from utils.agent_logger import end_pipeline, log_error, log_info, log_step, log_warning, start_pipeline

client = AsyncAzureOpenAI()

set_default_openai_client(client)
set_default_openai_api("chat_completions") 

set_tracing_disabled(True)


async def run_single_experiment() -> bool:
    """Run single experiment loop - using pipeline categorized logging."""
    # Start a new pipeline process
    pipeline_id = start_pipeline("experiment")
    
    try:
        # Step 1: Program sampling
        log_step("Program Sampling", "Start sampling program from database")
        context, parent = await program_sample()
        log_info(f"Program sampling completed, context length: {len(str(context))}")
        
        # Step 2: Evolution
        log_step("Program Evolution", "Start evolving new program")
        name, motivation = await evolve(context)
        if name == "Failed":
            log_error("Program evolution failed")
            end_pipeline(False, "Evolution failed")
            return False
        log_info(f"Program evolution successful, generated program: {name}")
        log_info(f"Evolution motivation: {motivation}")
        
        # Step 3: Evaluation
        log_step("Program Evaluation", f"Start evaluating program {name}")
        success = await evaluation(name, motivation)
        if not success:
            log_error(f"Program {name} evaluation failed")
            end_pipeline(False, "Evaluation failed")
            return False
        log_info(f"Program {name} evaluation successful")
        
        # Step 4: Analysis
        log_step("Result Analysis", f"Start analyzing program {name} results")
        result = await analyse(name, motivation, parent=parent)
        log_info(f"Analysis completed, result: {result}")
        
        # Step 5: Update database
        log_step("Database Update", "Update results to database")
        update(result)
        log_info("Database update completed")
        
        # Successfully complete pipeline
        log_info("Experiment pipeline completed successfully")
        end_pipeline(True, f"Experiment completed successfully, program: {name}, result: {result}")
        return True
        
    except KeyboardInterrupt:
        log_warning("User interrupted experiment")
        end_pipeline(False, "User interrupted experiment")
        return False
    except Exception as e:
        log_error(f"Experiment pipeline unexpected error: {str(e)}")
        end_pipeline(False, f"Unexpected error: {str(e)}")
        return False


async def main():
    """Main function - continuous experiment execution."""
    set_tracing_disabled(True)
    
    log_info("Starting continuous experiment pipeline...")
    
    # Run plot.py first
    log_info("Running plot scripts...")
    log_info("Plot scripts completed")
    
    experiment_count = 0
    while True:
        try:
            experiment_count += 1
            log_info(f"Starting experiment {experiment_count}")
            
            success = await run_single_experiment()
            if success:
                log_info(f"Experiment {experiment_count} completed successfully, starting next experiment...")
            else:
                log_warning(f"Experiment {experiment_count} failed, retrying in 60 seconds...")
                await asyncio.sleep(60)
                
        except KeyboardInterrupt:
            log_warning("Continuous experiment interrupted by user")
            break
        except Exception as e:
            log_error(f"Main loop unexpected error: {e}")
            log_info("Retrying in 60 seconds...")
            await asyncio.sleep(60)


if __name__ == "__main__":
    asyncio.run(main())